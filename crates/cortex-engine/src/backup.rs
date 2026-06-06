use std::fs::{self, File};
use std::io::{ErrorKind, Read, Write};
use std::path::{Path, PathBuf};

use cortex_storage::wal::WalWriter;

use crate::database::Database;
use crate::error::{EngineError, EngineResult};
use crate::validation::StorageValidation;

mod dry_run;
mod encrypted;
mod offsite;
mod retention;
pub use dry_run::RestoreDryRunReport;
pub use encrypted::{EncryptedBackupReport, EncryptedRestoreReport};
pub use offsite::OffsiteBackupStageReport;
pub use retention::{BackupRetentionPlan, BackupRetentionReport};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BackupReport {
    pub files_copied: usize,
    pub bytes_copied: u64,
    pub source_validation: StorageValidation,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RestoreReport {
    pub files_copied: usize,
    pub bytes_copied: u64,
    pub restored_validation: StorageValidation,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BackupDrillReport {
    pub backup: BackupReport,
    pub restore: RestoreReport,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct CopyReport {
    pub(super) files_copied: usize,
    pub(super) bytes_copied: u64,
}

impl Database {
    pub fn backup_path(
        source_path: impl AsRef<Path>,
        backup_path: impl AsRef<Path>,
    ) -> EngineResult<BackupReport> {
        let mut db = Database::open(source_path)?;
        let report = db.backup_to(backup_path)?;
        db.close()?;
        Ok(report)
    }

    pub fn backup_to(&mut self, backup_path: impl AsRef<Path>) -> EngineResult<BackupReport> {
        let source_validation = self.validate_storage()?;
        self.writer.shutdown()?;

        let backup_result = copy_database_dir(&self.root_path, backup_path.as_ref());
        let restart_result = WalWriter::start(&self.wal_path, self.durability_mode);
        self.writer = restart_result?;

        let copied = backup_result?;
        Ok(BackupReport {
            files_copied: copied.files_copied,
            bytes_copied: copied.bytes_copied,
            source_validation,
        })
    }

    pub fn restore_from_backup(
        backup_path: impl AsRef<Path>,
        target_path: impl AsRef<Path>,
    ) -> EngineResult<RestoreReport> {
        let copied = copy_database_dir(backup_path.as_ref(), target_path.as_ref())?;
        let db = Database::open(target_path)?;
        let restored_validation = db.validate_storage()?;
        db.close()?;
        Ok(RestoreReport {
            files_copied: copied.files_copied,
            bytes_copied: copied.bytes_copied,
            restored_validation,
        })
    }

    pub fn restore_from_backup_dry_run(
        backup_path: impl AsRef<Path>,
        target_path: impl AsRef<Path>,
    ) -> EngineResult<RestoreDryRunReport> {
        dry_run::restore_from_backup_dry_run(backup_path.as_ref(), target_path.as_ref())
    }

    pub fn backup_restore_drill_path(
        source_path: impl AsRef<Path>,
        backup_path: impl AsRef<Path>,
        restore_path: impl AsRef<Path>,
    ) -> EngineResult<BackupDrillReport> {
        let backup = Self::backup_path(source_path, backup_path.as_ref())?;
        let restore = Self::restore_from_backup(backup_path, restore_path)?;
        Ok(BackupDrillReport { backup, restore })
    }
}

pub(super) fn copy_database_dir(source: &Path, target: &Path) -> EngineResult<CopyReport> {
    let source = source.canonicalize()?;
    reject_target_inside_source(&source, target)?;
    reject_existing_target(target)?;
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::create_dir(target)?;
    let result = copy_dir_contents(&source, target);
    if result.is_err() {
        let _ = fs::remove_dir_all(target);
    }
    result
}

pub(super) fn reject_target_inside_source(source: &Path, target: &Path) -> EngineResult<()> {
    let target_abs = absolute_target_path(target)?;
    if target_abs.starts_with(source) {
        return Err(EngineError::StorageInvariant(format!(
            "backup target must not be inside source database: {}",
            target.display()
        )));
    }
    Ok(())
}

fn absolute_target_path(target: &Path) -> EngineResult<PathBuf> {
    if target.exists() {
        return Ok(target.canonicalize()?);
    }
    let parent = target.parent().unwrap_or_else(|| Path::new("."));
    let file_name = target.file_name().ok_or_else(|| {
        EngineError::StorageInvariant(format!("invalid backup path: {}", target.display()))
    })?;
    Ok(parent.canonicalize()?.join(file_name))
}

pub(super) fn reject_existing_target(target: &Path) -> EngineResult<()> {
    match fs::metadata(target) {
        Ok(_) => Err(EngineError::BackupTargetExists(target.to_owned())),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}

fn copy_dir_contents(source: &Path, target: &Path) -> EngineResult<CopyReport> {
    let mut report = CopyReport::default();
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if should_skip_backup_entry(&name_str) {
            continue;
        }
        let source_path = entry.path();
        let target_path = target.join(&name);
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            fs::create_dir(&target_path)?;
            let child = copy_dir_contents(&source_path, &target_path)?;
            report.files_copied += child.files_copied;
            report.bytes_copied += child.bytes_copied;
            sync_dir(&target_path)?;
        } else if file_type.is_file() {
            let bytes = copy_file_synced(&source_path, &target_path)?;
            report.files_copied += 1;
            report.bytes_copied += bytes;
        } else {
            return Err(EngineError::StorageInvariant(format!(
                "backup refuses non-regular file: {}",
                source_path.display()
            )));
        }
    }
    sync_dir(target)?;
    Ok(report)
}

pub(super) fn should_skip_backup_entry(name: &str) -> bool {
    name == "db.lock" || is_known_temp_name(name)
}

fn is_known_temp_name(name: &str) -> bool {
    [
        ".acs.tmp",
        ".acb.tmp",
        ".aci.tmp",
        ".acv.tmp",
        ".ach.tmp",
        ".acm.tmp",
        ".aclog.tmp",
        ".view.tmp",
    ]
    .iter()
    .any(|marker| name.ends_with(marker) || name.contains(&format!("{marker}.")))
}

fn copy_file_synced(source: &Path, target: &Path) -> EngineResult<u64> {
    let mut input = File::open(source)?;
    let mut output = File::create(target)?;
    let mut buffer = [0_u8; 64 * 1024];
    let mut bytes = 0_u64;
    loop {
        let read = input.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        output.write_all(&buffer[..read])?;
        bytes += read as u64;
    }
    output.sync_all()?;
    Ok(bytes)
}

pub(super) fn sync_dir(path: &Path) -> EngineResult<()> {
    File::open(path)?.sync_all()?;
    Ok(())
}
