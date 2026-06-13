use cortex_core::{CellDescriptor, CellId, CommitSeq};

use super::Database;
use crate::error::{EngineError, EngineResult};
use crate::feedback::FeedbackIndex;
use crate::operation::{
    wal_record_from_operation_with_metadata, wal_record_from_operation_with_seq, DbOperation,
};
use crate::query::CellMetadata;
use crate::session::SessionIndex;
use crate::tool_registry::ToolIndex;

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
        if cells.is_empty() {
            return Ok(self.current_seq);
        }
        let mut records = Vec::with_capacity(cells.len());
        let mut ops = Vec::with_capacity(cells.len());
        let mut next_seq = self.current_seq.0;

        for (cell_id, payload) in cells {
            next_seq += 1;
            let op = DbOperation::PutCell { cell_id, payload };
            let record = wal_record_from_operation_with_seq(CommitSeq(next_seq), &op);
            records.push(record);
            ops.push((CommitSeq(next_seq), op));
        }

        self.writer.append_batch(records)?;

        for (seq, op) in ops {
            self.apply_operation(seq, op)?;
        }
        self.current_seq = CommitSeq(next_seq);
        Ok(self.current_seq)
    }

    pub fn patch_cell(&mut self, cell_id: CellId, payload: Vec<u8>) -> EngineResult<CommitSeq> {
        self.require_visible_cell(cell_id)?;
        self.append_then_apply(DbOperation::PatchCell { cell_id, payload })
    }

    pub fn tombstone_cell(&mut self, cell_id: CellId) -> EngineResult<CommitSeq> {
        self.require_visible_cell(cell_id)?;
        self.append_then_apply(DbOperation::TombstoneCell { cell_id })
    }

    pub fn allocate_cell_id(&self) -> CellId {
        let max_id = self.memtable.max_cell_id().map(|id| id.0).unwrap_or(0);
        CellId(max_id.max(1000) + 1)
    }

    pub fn allocate_cell_id_range(&self, _count: usize) -> CellId {
        let max_id = self.memtable.max_cell_id().map(|id| id.0).unwrap_or(0);
        CellId(max_id.max(10000) + 1)
    }

    fn append_then_apply(&mut self, operation: DbOperation) -> EngineResult<CommitSeq> {
        let next_seq = CommitSeq(self.current_seq.0 + 1);
        let record = wal_record_from_operation_with_seq(next_seq, &operation);
        self.writer.append(record)?;
        self.apply_operation(next_seq, operation)?;
        self.current_seq = next_seq;
        Ok(next_seq)
    }

    pub(crate) fn append_then_apply_with_metadata(
        &mut self,
        operation: DbOperation,
        metadata: Vec<u8>,
    ) -> EngineResult<CommitSeq> {
        let next_seq = CommitSeq(self.current_seq.0 + 1);
        let descriptor =
            (!metadata.is_empty()).then(|| CellDescriptor::from_metadata_section_lossy(&metadata));
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
                let metadata = CellMetadata::from_payload_with_descriptor(&payload, &descriptor);
                let feedback_record = FeedbackIndex::record_from_payload(&payload);
                let session_record =
                    SessionIndex::record_from_payload(cell_id, &payload, &descriptor);
                let tool_record =
                    ToolIndex::record_from_payload(cell_id, seq, &payload, &descriptor);
                self.memtable
                    .put_cell_with_descriptor(cell_id, seq, payload, descriptor);
                self.aql_delta_index.apply_metadata(cell_id, metadata);
                self.feedback_index.apply_record(cell_id, feedback_record);
                self.session_index.apply_record(cell_id, session_record);
                self.tool_index.apply_record(cell_id, tool_record);
                Ok(())
            }
            DbOperation::PatchCell { cell_id, payload } => {
                let descriptor =
                    descriptor.unwrap_or_else(|| CellDescriptor::from_payload_lossy(&payload));
                let metadata = CellMetadata::from_payload_with_descriptor(&payload, &descriptor);
                let feedback_record = FeedbackIndex::record_from_payload(&payload);
                let session_record =
                    SessionIndex::record_from_payload(cell_id, &payload, &descriptor);
                let tool_record =
                    ToolIndex::record_from_payload(cell_id, seq, &payload, &descriptor);
                self.memtable
                    .patch_cell_with_descriptor(cell_id, seq, payload, descriptor)
                    .map_err(EngineError::from)?;
                self.aql_delta_index.apply_metadata(cell_id, metadata);
                self.feedback_index.apply_record(cell_id, feedback_record);
                self.session_index.apply_record(cell_id, session_record);
                self.tool_index.apply_record(cell_id, tool_record);
                Ok(())
            }
            DbOperation::TombstoneCell { cell_id } => {
                self.memtable.record_tombstone(cell_id, seq);
                self.aql_delta_index.apply_tombstone(cell_id);
                self.feedback_index.apply_tombstone(cell_id);
                self.session_index.apply_tombstone(cell_id);
                self.tool_index.apply_tombstone(cell_id);
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
}
