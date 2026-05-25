use std::io::ErrorKind;
use std::path::Path;

use cortex_core::memtable::MemTable;
use cortex_core::CommitSeq;
use cortex_storage::wal::WalReader;

use crate::error::EngineResult;
use crate::operation::{decoded_operation_from_wal_record, DbOperation};

#[derive(Clone, Debug)]
pub struct ReplayResult {
    pub memtable: MemTable,
    pub last_seq: CommitSeq,
    pub records_replayed: usize,
    pub safe_truncate_offset: u64,
}

pub fn replay_wal(path: &Path) -> EngineResult<ReplayResult> {
    let scan = match WalReader::scan_path(path) {
        Ok(scan) => scan,
        Err(cortex_storage::StorageError::Io(error)) if error.kind() == ErrorKind::NotFound => {
            return Ok(empty_replay());
        }
        Err(error) => return Err(error.into()),
    };
    replay_scan(scan)
}

pub fn replay_wal_best_effort(path: &Path) -> EngineResult<ReplayResult> {
    let scan = match WalReader::scan_best_effort_path(path) {
        Ok(scan) => scan,
        Err(cortex_storage::StorageError::Io(error)) if error.kind() == ErrorKind::NotFound => {
            return Ok(empty_replay());
        }
        Err(error) => return Err(error.into()),
    };
    replay_scan(scan)
}

fn replay_scan(scan: cortex_storage::wal::WalScan) -> EngineResult<ReplayResult> {
    let mut memtable = MemTable::default();
    let mut last_seq = CommitSeq(0);
    for (index, record) in scan.records.iter().enumerate() {
        let decoded = decoded_operation_from_wal_record(record)?;
        let seq = decoded.seq.unwrap_or(CommitSeq(index as u64 + 1));
        last_seq = last_seq.max(seq);
        match decoded.operation {
            DbOperation::PutCell { cell_id, payload } => {
                memtable.put_cell(cell_id, seq, payload);
            }
            DbOperation::PatchCell { cell_id, payload } => {
                memtable.patch_cell(cell_id, seq, payload)?;
            }
            DbOperation::TombstoneCell { cell_id } => {
                memtable.tombstone_cell(cell_id, seq)?;
            }
        }
    }
    Ok(ReplayResult {
        memtable,
        last_seq,
        records_replayed: scan.records.len(),
        safe_truncate_offset: scan.safe_truncate_offset,
    })
}

fn empty_replay() -> ReplayResult {
    ReplayResult {
        memtable: MemTable::default(),
        last_seq: CommitSeq(0),
        records_replayed: 0,
        safe_truncate_offset: 0,
    }
}
