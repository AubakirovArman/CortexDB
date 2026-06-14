use std::collections::BTreeSet;

use cortex_core::{CellDescriptor, CellId, CommitSeq};

use super::Database;
use crate::cell_ids::GENERIC_CELL_ID_FLOOR;
use crate::error::{EngineError, EngineResult};
use crate::operation::{
    wal_record_from_operation_with_metadata, wal_record_from_operation_with_seq,
    wal_record_from_write_batch_begin, wal_record_from_write_batch_commit, DbOperation, WriteBatch,
    WriteBatchOperation,
};
use crate::operation_descriptor::descriptor_from_operation_with_metadata;

impl Database {
    /// Store a single cell payload and return the commit sequence.
    ///
    /// # Example
    ///
    /// ```
    /// # use cortex_engine::Database;
    /// # use cortex_core::CellId;
    /// # let dir = tempfile::tempdir().unwrap();
    /// # let mut db = Database::open(dir.path()).unwrap();
    /// let seq = db.put_cell(CellId(1), b"hello world".to_vec()).unwrap();
    /// assert!(seq.0 > 0);
    /// ```
    pub fn put_cell(&mut self, cell_id: CellId, payload: Vec<u8>) -> EngineResult<CommitSeq> {
        self.append_then_apply(DbOperation::PutCell { cell_id, payload })
    }

    pub fn put_cells(&mut self, cells: Vec<(CellId, Vec<u8>)>) -> EngineResult<CommitSeq> {
        let operations = cells
            .into_iter()
            .map(|(cell_id, payload)| DbOperation::PutCell { cell_id, payload })
            .collect();
        self.append_then_apply_batch(operations)
    }

    pub fn write_batch(&mut self, batch: WriteBatch) -> EngineResult<CommitSeq> {
        self.validate_write_batch(&batch)?;
        let operations = batch
            .into_operations()
            .into_iter()
            .map(DbOperation::from)
            .collect();
        self.append_then_apply_batch(operations)
    }

    pub fn patch_cell(&mut self, cell_id: CellId, payload: Vec<u8>) -> EngineResult<CommitSeq> {
        self.require_visible_cell(cell_id)?;
        self.append_then_apply(DbOperation::PatchCell { cell_id, payload })
    }

    pub fn tombstone_cell(&mut self, cell_id: CellId) -> EngineResult<CommitSeq> {
        self.require_visible_cell(cell_id)?;
        self.append_then_apply(DbOperation::TombstoneCell { cell_id })
    }

    pub fn allocate_cell_id(&mut self) -> EngineResult<CellId> {
        self.allocate_cell_id_range(1)
    }

    pub fn allocate_cell_id_range(&mut self, count: usize) -> EngineResult<CellId> {
        let count = u64::try_from(count)
            .map_err(|_| EngineError::StorageInvariant("cell id range is too large".to_owned()))?;
        let observed_next = self.observed_next_generic_cell_id();
        let first = self
            .manifest
            .reserve_cell_id_range(GENERIC_CELL_ID_FLOOR, observed_next, count)
            .ok_or_else(|| {
                EngineError::StorageInvariant("cell id space is exhausted".to_owned())
            })?;
        self.manifest.store(&self.manifest_path)?;
        Ok(CellId(first))
    }

    fn observed_next_generic_cell_id(&self) -> u64 {
        let max_live = self
            .memtable
            .live_cell_ids(self.read_txn())
            .into_iter()
            .map(|cell_id| cell_id.0)
            .filter(|cell_id| *cell_id < crate::cell_ids::MEMORY_CELL_NAMESPACE)
            .max();
        let max_deleted = self
            .memtable
            .deleted_cell_ids()
            .into_iter()
            .map(|cell_id| cell_id.0)
            .filter(|cell_id| *cell_id < crate::cell_ids::MEMORY_CELL_NAMESPACE)
            .max();
        max_live
            .into_iter()
            .chain(max_deleted)
            .max()
            .and_then(|max_id| max_id.checked_add(1))
            .unwrap_or(GENERIC_CELL_ID_FLOOR + 1)
    }

    fn append_then_apply(&mut self, operation: DbOperation) -> EngineResult<CommitSeq> {
        let next_seq = CommitSeq(self.current_seq.0 + 1);
        let record = wal_record_from_operation_with_seq(next_seq, &operation);
        self.writer.append(record)?;
        self.apply_operation(next_seq, operation)?;
        self.current_seq = next_seq;
        Ok(next_seq)
    }

    fn append_then_apply_batch(&mut self, operations: Vec<DbOperation>) -> EngineResult<CommitSeq> {
        if operations.is_empty() {
            return Ok(self.current_seq);
        }
        if operations.len() == 1 {
            if let Some(operation) = operations.into_iter().next() {
                return self.append_then_apply(operation);
            }
            return Err(EngineError::StorageInvariant(
                "write batch operation disappeared before WAL append".to_owned(),
            ));
        }

        let operation_count = u32::try_from(operations.len())
            .map_err(|_| EngineError::StorageInvariant("write batch is too large".to_owned()))?;
        let start_seq = CommitSeq(self.current_seq.0 + 1);
        let end_seq = CommitSeq(self.current_seq.0 + u64::from(operation_count));
        let mut records = Vec::with_capacity(operations.len() + 2);
        let mut sequenced_operations = Vec::with_capacity(operations.len());
        let mut next_seq = self.current_seq.0;

        records.push(wal_record_from_write_batch_begin(
            start_seq,
            end_seq,
            operation_count,
        ));
        for operation in operations {
            next_seq += 1;
            let seq = CommitSeq(next_seq);
            let record = wal_record_from_operation_with_seq(seq, &operation);
            records.push(record);
            sequenced_operations.push((seq, operation));
        }
        records.push(wal_record_from_write_batch_commit(
            start_seq,
            end_seq,
            operation_count,
        ));

        self.writer.append_batch(records)?;

        for (seq, operation) in sequenced_operations {
            self.apply_operation(seq, operation)?;
        }
        self.current_seq = CommitSeq(next_seq);
        Ok(self.current_seq)
    }

    pub(crate) fn append_then_apply_with_metadata(
        &mut self,
        operation: DbOperation,
        metadata: Vec<u8>,
    ) -> EngineResult<CommitSeq> {
        let next_seq = CommitSeq(self.current_seq.0 + 1);
        let descriptor = descriptor_from_operation_with_metadata(&operation, &metadata);
        let record = wal_record_from_operation_with_metadata(next_seq, &operation, metadata);
        self.writer.append(record)?;
        self.apply_operation_with_descriptor(next_seq, operation, descriptor)?;
        self.current_seq = next_seq;
        Ok(next_seq)
    }

    fn apply_operation(&mut self, seq: CommitSeq, operation: DbOperation) -> EngineResult<()> {
        self.apply_operation_with_descriptor(seq, operation, None)
    }

    fn apply_operation_with_descriptor(
        &mut self,
        seq: CommitSeq,
        operation: DbOperation,
        descriptor: Option<CellDescriptor>,
    ) -> EngineResult<()> {
        match operation {
            DbOperation::PutCell { cell_id, payload } => {
                let descriptor =
                    descriptor.unwrap_or_else(|| CellDescriptor::from_payload_lossy(&payload));
                self.memtable.put_cell_with_descriptor(
                    cell_id,
                    seq,
                    payload.clone(),
                    descriptor.clone(),
                );
                self.apply_derived_cell_record(cell_id, seq, &payload, &descriptor);
                Ok(())
            }
            DbOperation::PatchCell { cell_id, payload } => {
                let descriptor =
                    descriptor.unwrap_or_else(|| CellDescriptor::from_payload_lossy(&payload));
                self.memtable
                    .patch_cell_with_descriptor(cell_id, seq, payload.clone(), descriptor.clone())
                    .map_err(EngineError::from)?;
                self.apply_derived_cell_record(cell_id, seq, &payload, &descriptor);
                Ok(())
            }
            DbOperation::TombstoneCell { cell_id } => {
                self.memtable.record_tombstone(cell_id, seq);
                self.apply_derived_tombstone(cell_id);
                Ok(())
            }
        }
    }

    fn require_visible_cell(&self, cell_id: CellId) -> EngineResult<()> {
        self.memtable
            .read(self.read_txn(), cell_id)
            .map(|_| ())
            .ok_or_else(|| cortex_core::CoreError::CellNotFound(cell_id).into())
    }

    fn validate_write_batch(&self, batch: &WriteBatch) -> EngineResult<()> {
        let mut batch_puts = BTreeSet::new();
        let mut batch_tombstones = BTreeSet::new();
        for operation in batch.operations() {
            match operation {
                WriteBatchOperation::PutCell { cell_id, .. } => {
                    batch_tombstones.remove(cell_id);
                    batch_puts.insert(*cell_id);
                }
                WriteBatchOperation::PatchCell { cell_id, .. } => {
                    if !self.batch_sees_visible_cell(*cell_id, &batch_puts, &batch_tombstones) {
                        return Err(cortex_core::CoreError::CellNotFound(*cell_id).into());
                    }
                }
                WriteBatchOperation::TombstoneCell { cell_id } => {
                    if !self.batch_sees_visible_cell(*cell_id, &batch_puts, &batch_tombstones) {
                        return Err(cortex_core::CoreError::CellNotFound(*cell_id).into());
                    }
                    batch_puts.remove(cell_id);
                    batch_tombstones.insert(*cell_id);
                }
            }
        }
        Ok(())
    }

    fn batch_sees_visible_cell(
        &self,
        cell_id: CellId,
        batch_puts: &BTreeSet<CellId>,
        batch_tombstones: &BTreeSet<CellId>,
    ) -> bool {
        !batch_tombstones.contains(&cell_id)
            && (batch_puts.contains(&cell_id)
                || self.memtable.read(self.read_txn(), cell_id).is_some())
    }
}
