use std::fs;
use std::io::ErrorKind;
use std::path::Path;

use cortex_storage::wal::WalReader;

use crate::cleanup::cleanup_orphans;
use crate::database::{truncate_wal_tail, Database};
use crate::error::{EngineError, EngineResult};
use crate::lock::DatabaseLock;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RepairReport {
    pub orphan_temp_files_removed: usize,
    pub wal_records_preserved: usize,
    pub wal_safe_truncate_offset: u64,
    pub wal_bytes_before: u64,
    pub wal_bytes_after: u64,
    pub wal_truncated: bool,
}

impl Database {
    pub fn repair_best_effort(path: impl AsRef<Path>) -> EngineResult<RepairReport> {
        let root = path.as_ref();
        fs::create_dir_all(root)?;
        let _lock = DatabaseLock::acquire(root)?;
        let orphan_temp_files_removed = cleanup_orphans(root)?;
        let wal_path = root.join("db.aclog");
        let wal_bytes_before = file_len_or_zero(&wal_path)?;
        let scan = match WalReader::scan_best_effort_path(&wal_path) {
            Ok(scan) => scan,
            Err(cortex_storage::StorageError::Io(error)) if error.kind() == ErrorKind::NotFound => {
                return Ok(RepairReport {
                    orphan_temp_files_removed,
                    ..RepairReport::default()
                });
            }
            Err(error) => return Err(EngineError::from(error)),
        };
        truncate_wal_tail(&wal_path, scan.safe_truncate_offset)?;
        let wal_bytes_after = file_len_or_zero(&wal_path)?;
        Ok(RepairReport {
            orphan_temp_files_removed,
            wal_records_preserved: scan.records.len(),
            wal_safe_truncate_offset: scan.safe_truncate_offset,
            wal_bytes_before,
            wal_bytes_after,
            wal_truncated: wal_bytes_after < wal_bytes_before,
        })
    }
}

fn file_len_or_zero(path: &Path) -> EngineResult<u64> {
    match fs::metadata(path) {
        Ok(metadata) => Ok(metadata.len()),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(0),
        Err(error) => Err(error.into()),
    }
}
