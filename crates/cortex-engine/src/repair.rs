use std::fs;
use std::io::ErrorKind;
use std::path::Path;

use cortex_storage::wal::WalReader;

use crate::cleanup::{cleanup_orphans, count_orphans};
use crate::database::Database;
use crate::error::{EngineError, EngineResult};
use crate::lock::DatabaseLock;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RepairReport {
    pub dry_run: bool,
    pub orphan_temp_files_removed: usize,
    pub wal_records_preserved: usize,
    pub wal_safe_truncate_offset: u64,
    pub wal_bytes_before: u64,
    pub wal_bytes_after: u64,
    pub wal_truncated: bool,
    pub wal_truncation_needed: bool,
}

impl Database {
    pub fn repair_best_effort(path: impl AsRef<Path>) -> EngineResult<RepairReport> {
        repair_best_effort_inner(path.as_ref(), false)
    }

    pub fn repair_best_effort_dry_run(path: impl AsRef<Path>) -> EngineResult<RepairReport> {
        repair_best_effort_inner(path.as_ref(), true)
    }
}

fn repair_best_effort_inner(root: &Path, dry_run: bool) -> EngineResult<RepairReport> {
    fs::create_dir_all(root)?;
    let _lock = DatabaseLock::acquire(root)?;
    let orphan_temp_files_removed = if dry_run {
        count_orphans(root)?
    } else {
        cleanup_orphans(root)?
    };
    let wal_path = root.join("db.aclog");
    let wal_bytes_before = file_len_or_zero(&wal_path)?;
    let scan = match WalReader::scan_best_effort_path(&wal_path) {
        Ok(scan) => scan,
        Err(cortex_storage::StorageError::Io(error)) if error.kind() == ErrorKind::NotFound => {
            return Ok(RepairReport {
                dry_run,
                orphan_temp_files_removed,
                ..RepairReport::default()
            });
        }
        Err(error) => return Err(EngineError::from(error)),
    };
    let wal_truncation_needed = scan.safe_truncate_offset < wal_bytes_before;
    if !dry_run {
        crate::database_files::truncate_wal_tail(&wal_path, scan.safe_truncate_offset)?;
    }
    let wal_bytes_after = file_len_or_zero(&wal_path)?;
    Ok(RepairReport {
        dry_run,
        orphan_temp_files_removed,
        wal_records_preserved: scan.records.len(),
        wal_safe_truncate_offset: scan.safe_truncate_offset,
        wal_bytes_before,
        wal_bytes_after,
        wal_truncated: wal_bytes_after < wal_bytes_before,
        wal_truncation_needed,
    })
}

fn file_len_or_zero(path: &Path) -> EngineResult<u64> {
    match fs::metadata(path) {
        Ok(metadata) => Ok(metadata.len()),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(0),
        Err(error) => Err(error.into()),
    }
}
