use std::fs;
use std::path::{Path, PathBuf};

use crate::database::Database;
use crate::error::{EngineError, EngineResult};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BackupRetentionPlan {
    pub backup_root: PathBuf,
    pub prefix: String,
    pub keep_latest: usize,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BackupRetentionReport {
    pub backups_seen: usize,
    pub backups_kept: usize,
    pub backups_removed: usize,
    pub bytes_removed: u64,
}

impl Database {
    pub fn prune_backup_retention(
        backup_root: impl AsRef<Path>,
        prefix: impl AsRef<str>,
        keep_latest: usize,
    ) -> EngineResult<BackupRetentionReport> {
        let plan = BackupRetentionPlan {
            backup_root: backup_root.as_ref().to_owned(),
            prefix: prefix.as_ref().to_owned(),
            keep_latest,
        };
        prune_backup_retention_plan(&plan)
    }
}

fn prune_backup_retention_plan(plan: &BackupRetentionPlan) -> EngineResult<BackupRetentionReport> {
    validate_plan(plan)?;
    let mut backups = matching_backup_dirs(plan)?;
    backups.sort_by(|left, right| right.file_name.cmp(&left.file_name));

    let mut report = BackupRetentionReport {
        backups_seen: backups.len(),
        backups_kept: backups.len().min(plan.keep_latest),
        backups_removed: 0,
        bytes_removed: 0,
    };

    for backup in backups.into_iter().skip(plan.keep_latest) {
        report.bytes_removed += directory_size(&backup.path)?;
        fs::remove_dir_all(&backup.path)?;
        report.backups_removed += 1;
    }

    Ok(report)
}

fn validate_plan(plan: &BackupRetentionPlan) -> EngineResult<()> {
    if plan.prefix.trim().is_empty() {
        return Err(EngineError::StorageInvariant(
            "backup retention prefix must not be empty".to_owned(),
        ));
    }
    if plan.keep_latest == 0 {
        return Err(EngineError::StorageInvariant(
            "backup retention keep_latest must be greater than zero".to_owned(),
        ));
    }
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct BackupDir {
    file_name: String,
    path: PathBuf,
}

fn matching_backup_dirs(plan: &BackupRetentionPlan) -> EngineResult<Vec<BackupDir>> {
    let mut backups = Vec::new();
    for entry in fs::read_dir(&plan.backup_root)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        if !file_type.is_dir() {
            continue;
        }
        let file_name = entry.file_name().to_string_lossy().into_owned();
        if file_name.starts_with(&plan.prefix) {
            backups.push(BackupDir {
                file_name,
                path: entry.path(),
            });
        }
    }
    Ok(backups)
}

fn directory_size(path: &Path) -> EngineResult<u64> {
    let mut bytes = 0_u64;
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            bytes += directory_size(&entry.path())?;
        } else if file_type.is_file() {
            bytes += entry.metadata()?.len();
        }
    }
    Ok(bytes)
}
