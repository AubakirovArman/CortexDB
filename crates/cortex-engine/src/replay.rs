use std::io::ErrorKind;
use std::path::Path;

use cortex_core::memtable::MemTable;
use cortex_core::CommitSeq;
use cortex_storage::wal::WalReader;

use crate::error::EngineError;
use crate::error::EngineResult;
use crate::operation::{
    decoded_operation_from_wal_record, decoded_write_batch_marker, DbOperation, DecodedDbOperation,
    DecodedWriteBatchMarker, WriteBatchMarker,
};

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
    let mut pending_batch: Option<PendingWriteBatch> = None;
    let mut safe_truncate_offset = scan.safe_truncate_offset;
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
    for record in &scan.records {
        if let Some(marker) = decoded_write_batch_marker(record)? {
            match marker {
                DecodedWriteBatchMarker::Begin(marker) => {
                    if pending_batch.is_some() {
                        return Err(EngineError::StorageInvariant(
                            "nested WAL write batch marker".to_owned(),
                        ));
                    }
                    pending_batch = Some(PendingWriteBatch {
                        marker,
                        begin_lsn: record.lsn,
                        operations: Vec::new(),
                    });
                }
                DecodedWriteBatchMarker::Commit(marker) => {
                    let Some(batch) = pending_batch.take() else {
                        return Err(EngineError::StorageInvariant(
                            "WAL write batch commit without begin".to_owned(),
                        ));
                    };
                    validate_committed_batch(&batch, marker)?;
                    for decoded in batch.operations {
                        apply_decoded_operation(
                            &mut memtable,
                            base_seq,
                            &mut last_seq,
                            &mut records_replayed,
                            &mut metrics,
                            decoded,
                        )?;
                    }
                }
            }
            continue;
        }

        let decoded = decoded_operation_from_wal_record(record)?;
        if let Some(batch) = pending_batch.as_mut() {
            batch.operations.push(decoded);
            continue;
        }

        apply_decoded_operation(
            &mut memtable,
            base_seq,
            &mut last_seq,
            &mut records_replayed,
            &mut metrics,
            decoded,
        )?;
    }
    if let Some(batch) = pending_batch {
        safe_truncate_offset = batch.begin_lsn;
        metrics.safe_truncate_offset = safe_truncate_offset;
    } else {
        metrics.safe_truncate_offset = scan.safe_truncate_offset;
    }
    Ok(ReplayResult {
        memtable,
        last_seq,
        records_replayed,
        safe_truncate_offset,
        metrics,
    })
}

#[derive(Clone, Debug)]
struct PendingWriteBatch {
    marker: WriteBatchMarker,
    begin_lsn: u64,
    operations: Vec<DecodedDbOperation>,
}

fn validate_committed_batch(
    batch: &PendingWriteBatch,
    commit: WriteBatchMarker,
) -> EngineResult<()> {
    if batch.marker != commit {
        return Err(EngineError::StorageInvariant(
            "WAL write batch begin/commit marker mismatch".to_owned(),
        ));
    }
    let expected_len = usize::try_from(commit.operation_count).map_err(|_| {
        EngineError::StorageInvariant("WAL write batch operation count overflows usize".to_owned())
    })?;
    if batch.operations.len() != expected_len {
        return Err(EngineError::StorageInvariant(
            "WAL write batch operation count mismatch".to_owned(),
        ));
    }
    let mut expected_seq = commit.start_seq.0;
    for operation in &batch.operations {
        if operation.seq.0 != expected_seq || operation.seq > commit.end_seq {
            return Err(EngineError::StorageInvariant(
                "WAL write batch commit sequence range mismatch".to_owned(),
            ));
        }
        expected_seq += 1;
    }
    if expected_seq != commit.end_seq.0 + 1 {
        return Err(EngineError::StorageInvariant(
            "WAL write batch end sequence mismatch".to_owned(),
        ));
    }
    Ok(())
}

fn apply_decoded_operation(
    memtable: &mut MemTable,
    base_seq: CommitSeq,
    last_seq: &mut CommitSeq,
    records_replayed: &mut usize,
    metrics: &mut ReplayMetrics,
    decoded: DecodedDbOperation,
) -> EngineResult<()> {
    let seq = decoded.seq;
    if seq <= base_seq {
        metrics.records_skipped += 1;
        return Ok(());
    }
    *last_seq = (*last_seq).max(seq);
    *records_replayed += 1;
    metrics.records_applied += 1;
    match decoded.operation {
        DbOperation::PutCell { cell_id, payload } => {
            if let Some(descriptor) = decoded.descriptor {
                memtable.put_cell_with_descriptor(cell_id, seq, payload, descriptor);
            } else {
                memtable.put_cell(cell_id, seq, payload);
            }
        }
        DbOperation::PatchCell { cell_id, payload } => {
            if let Some(descriptor) = decoded.descriptor {
                memtable.patch_cell_with_descriptor(cell_id, seq, payload, descriptor)?;
            } else {
                memtable.patch_cell(cell_id, seq, payload)?;
            }
        }
        DbOperation::TombstoneCell { cell_id } => {
            memtable.record_tombstone(cell_id, seq);
        }
    }
    Ok(())
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
