use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use crate::backup::{copy_database_dir, sync_dir, RestoreReport};
use crate::database::Database;
use crate::error::{EngineError, EngineResult};
use crate::validation::StorageValidation;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OffsiteBackupStageReport {
    pub target_path: PathBuf,
    pub files_copied: usize,
    pub bytes_copied: u64,
    pub drill_restore: RestoreReport,
    pub staged_validation: StorageValidation,
}

impl Database {
    pub fn stage_backup_offsite(
        backup_path: impl AsRef<Path>,
        offsite_root: impl AsRef<Path>,
        backup_id: &str,
    ) -> EngineResult<OffsiteBackupStageReport> {
        validate_backup_id(backup_id)?;
        fs::create_dir_all(offsite_root.as_ref())?;

        let final_path = offsite_root.as_ref().join(backup_id);
        let staging_path = offsite_root.as_ref().join(format!("{backup_id}.staging"));
        let drill_path = offsite_root
            .as_ref()
            .join(format!("{backup_id}.preflight-restore"));

        reject_existing(&final_path)?;
        reject_existing(&staging_path)?;
        reject_existing(&drill_path)?;

        let drill_restore = match Self::restore_from_backup(backup_path.as_ref(), &drill_path) {
            Ok(report) => report,
            Err(error) => {
                let _ = fs::remove_dir_all(&drill_path);
                return Err(error);
            }
        };
        fs::remove_dir_all(&drill_path)?;

        let copied = match copy_database_dir(backup_path.as_ref(), &staging_path) {
            Ok(report) => report,
            Err(error) => {
                let _ = fs::remove_dir_all(&staging_path);
                return Err(error);
            }
        };
        let staged_validation = match validate_staged_copy(&staging_path) {
            Ok(validation) => validation,
            Err(error) => {
                let _ = fs::remove_dir_all(&staging_path);
                return Err(error);
            }
        };

        fs::rename(&staging_path, &final_path)?;
        sync_dir(offsite_root.as_ref())?;

        Ok(OffsiteBackupStageReport {
            target_path: final_path,
            files_copied: copied.files_copied,
            bytes_copied: copied.bytes_copied,
            drill_restore,
            staged_validation,
        })
    }
}

fn validate_staged_copy(path: &Path) -> EngineResult<StorageValidation> {
    let db = Database::open(path)?;
    let validation = db.validate_storage()?;
    db.close()?;
    Ok(validation)
}

fn reject_existing(path: &Path) -> EngineResult<()> {
    match fs::metadata(path) {
        Ok(_) => Err(EngineError::BackupTargetExists(path.to_owned())),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}

fn validate_backup_id(backup_id: &str) -> EngineResult<()> {
    let valid = !backup_id.is_empty()
        && backup_id.len() <= 128
        && backup_id != "."
        && backup_id != ".."
        && backup_id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.');
    if valid {
        Ok(())
    } else {
        Err(EngineError::StorageInvariant(
            "offsite backup id must be 1-128 chars of ASCII letters, digits, '.', '_' or '-'"
                .to_owned(),
        ))
    }
}
