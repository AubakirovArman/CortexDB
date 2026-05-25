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
    pub metrics: ReplayMetrics,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ReplayMetrics {
    pub records_seen: usize,
    pub records_applied: usize,
    pub records_skipped: usize,
    pub payload_bytes: u64,
    pub safe_truncate_offset: u64,
}

pub fn replay_wal(path: &Path) -> EngineResult<ReplayResult> {
    replay_wal_into(path, MemTable::default(), CommitSeq(0))
}

pub fn replay_wal_best_effort(path: &Path) -> EngineResult<ReplayResult> {
    replay_wal_best_effort_into(path, MemTable::default(), CommitSeq(0))
}

pub fn replay_wal_into(
    path: &Path,
    base_memtable: MemTable,
    base_seq: CommitSeq,
) -> EngineResult<ReplayResult> {
    let scan = match WalReader::scan_path(path) {
        Ok(scan) => scan,
        Err(cortex_storage::StorageError::Io(error)) if error.kind() == ErrorKind::NotFound => {
            return Ok(empty_replay(base_memtable, base_seq));
        }
        Err(error) => return Err(error.into()),
    };
    replay_scan(scan, base_memtable, base_seq)
}

pub fn replay_wal_best_effort_into(
    path: &Path,
    base_memtable: MemTable,
    base_seq: CommitSeq,
) -> EngineResult<ReplayResult> {
    let scan = match WalReader::scan_best_effort_path(path) {
        Ok(scan) => scan,
        Err(cortex_storage::StorageError::Io(error)) if error.kind() == ErrorKind::NotFound => {
            return Ok(empty_replay(base_memtable, base_seq));
        }
        Err(error) => return Err(error.into()),
    };
    replay_scan(scan, base_memtable, base_seq)
}

fn replay_scan(
    scan: cortex_storage::wal::WalScan,
    mut memtable: MemTable,
    base_seq: CommitSeq,
) -> EngineResult<ReplayResult> {
    let mut last_seq = base_seq;
    let mut records_replayed = 0;
    let mut metrics = ReplayMetrics {
        records_seen: scan.records.len(),
        payload_bytes: scan
            .records
            .iter()
            .map(|record| u64::from(record.header.payload_len))
            .sum(),
        safe_truncate_offset: scan.safe_truncate_offset,
        ..ReplayMetrics::default()
    };
    for (index, record) in scan.records.iter().enumerate() {
        let decoded = decoded_operation_from_wal_record(record)?;
        let seq = decoded
            .seq
            .unwrap_or(CommitSeq(base_seq.0 + index as u64 + 1));
        if seq <= base_seq {
            metrics.records_skipped += 1;
            continue;
        }
        last_seq = last_seq.max(seq);
        records_replayed += 1;
        metrics.records_applied += 1;
        match decoded.operation {
            DbOperation::PutCell { cell_id, payload } => {
                memtable.put_cell(cell_id, seq, payload);
            }
            DbOperation::PatchCell { cell_id, payload } => {
                memtable.patch_cell(cell_id, seq, payload)?;
            }
            DbOperation::TombstoneCell { cell_id } => {
                memtable.record_tombstone(cell_id, seq);
            }
        }
    }
    Ok(ReplayResult {
        memtable,
        last_seq,
        records_replayed,
        safe_truncate_offset: scan.safe_truncate_offset,
        metrics,
    })
}

fn empty_replay(memtable: MemTable, last_seq: CommitSeq) -> ReplayResult {
    ReplayResult {
        memtable,
        last_seq,
        records_replayed: 0,
        safe_truncate_offset: 0,
        metrics: ReplayMetrics::default(),
    }
}
