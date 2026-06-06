use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use crate::backup::{copy_database_dir, sync_dir, RestoreReport};
use crate::database::Database;
use crate::error::{EngineError, EngineResult};
use crate::validation::StorageValidation;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OffsiteBackupStageReport {
    pub adapter: &'static str,
    pub target_path: PathBuf,
    pub staging_path: PathBuf,
    pub preflight_restore_path: PathBuf,
    pub files_copied: usize,
    pub bytes_copied: u64,
    pub drill_restore: RestoreReport,
    pub staged_validation: StorageValidation,
    pub published: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct OffsiteBackupTransferReport {
    pub files_copied: usize,
    pub bytes_copied: u64,
}

pub trait OffsiteBackupAdapter {
    fn name(&self) -> &'static str;

    fn stage_backup(
        &self,
        backup_path: &Path,
        staging_path: &Path,
    ) -> EngineResult<OffsiteBackupTransferReport>;

    fn validate_staged_backup(&self, staging_path: &Path) -> EngineResult<StorageValidation>;

    fn publish_staged_backup(&self, staging_path: &Path, final_path: &Path) -> EngineResult<()>;
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LocalFilesystemOffsiteAdapter;

impl OffsiteBackupAdapter for LocalFilesystemOffsiteAdapter {
    fn name(&self) -> &'static str {
        "local_filesystem"
    }

    fn stage_backup(
        &self,
        backup_path: &Path,
        staging_path: &Path,
    ) -> EngineResult<OffsiteBackupTransferReport> {
        let report = copy_database_dir(backup_path, staging_path)?;
        Ok(OffsiteBackupTransferReport {
            files_copied: report.files_copied,
            bytes_copied: report.bytes_copied,
        })
    }

    fn validate_staged_backup(&self, staging_path: &Path) -> EngineResult<StorageValidation> {
        validate_staged_copy(staging_path)
    }

    fn publish_staged_backup(&self, staging_path: &Path, final_path: &Path) -> EngineResult<()> {
        fs::rename(staging_path, final_path)?;
        if let Some(parent) = final_path.parent() {
            sync_dir(parent)?;
        }
        Ok(())
    }
}

impl Database {
    pub fn stage_backup_offsite(
        backup_path: impl AsRef<Path>,
        offsite_root: impl AsRef<Path>,
        backup_id: &str,
    ) -> EngineResult<OffsiteBackupStageReport> {
        Self::stage_backup_offsite_with_adapter(
            backup_path,
            offsite_root,
            backup_id,
            &LocalFilesystemOffsiteAdapter,
        )
    }

    pub fn stage_backup_offsite_with_adapter<A: OffsiteBackupAdapter>(
        backup_path: impl AsRef<Path>,
        offsite_root: impl AsRef<Path>,
        backup_id: &str,
        adapter: &A,
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

        let copied = match adapter.stage_backup(backup_path.as_ref(), &staging_path) {
            Ok(report) => report,
            Err(error) => {
                let _ = fs::remove_dir_all(&staging_path);
                return Err(error);
            }
        };
        let staged_validation = match adapter.validate_staged_backup(&staging_path) {
            Ok(validation) => validation,
            Err(error) => {
                let _ = fs::remove_dir_all(&staging_path);
                return Err(error);
            }
        };

        adapter.publish_staged_backup(&staging_path, &final_path)?;

        Ok(OffsiteBackupStageReport {
            adapter: adapter.name(),
            target_path: final_path,
            staging_path,
            preflight_restore_path: drill_path,
            files_copied: copied.files_copied,
            bytes_copied: copied.bytes_copied,
            drill_restore,
            staged_validation,
            published: true,
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
