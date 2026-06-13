use std::collections::BTreeSet;

use cortex_core::{CellDescriptor, CellId, CommitSeq};

use super::Database;
use crate::error::{EngineError, EngineResult};
use crate::feedback::FeedbackIndex;
use crate::graph::GraphIndexStore;
use crate::operation::{
    wal_record_from_operation_with_metadata, wal_record_from_operation_with_seq,
    wal_record_from_write_batch_begin, wal_record_from_write_batch_commit, DbOperation, WriteBatch,
    WriteBatchOperation,
};
use crate::query::CellMetadata;
use crate::search::{CorpusSynonymStore, LiveSearchStore, SearchContextStore};
use crate::session::SessionIndex;
use crate::tool_registry::ToolIndex;
use crate::verification::TemporalFactStore;

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
                let corpus_synonym_record =
                    CorpusSynonymStore::record_from_payload(cell_id, &payload);
                let graph_record =
                    GraphIndexStore::record_from_payload(payload.clone(), &descriptor);
                let live_search_record =
                    LiveSearchStore::record_from_payload(payload.clone(), &descriptor);
                let search_context_record =
                    SearchContextStore::record_from_payload(payload.clone(), &descriptor);
                let feedback_record = FeedbackIndex::record_from_payload(&payload);
                let session_record =
                    SessionIndex::record_from_payload(cell_id, &payload, &descriptor);
                let temporal_record =
                    TemporalFactStore::record_from_payload(cell_id, &payload, &descriptor);
                let tool_record =
                    ToolIndex::record_from_payload(cell_id, seq, &payload, &descriptor);
                self.memtable
                    .put_cell_with_descriptor(cell_id, seq, payload, descriptor);
                self.aql_delta_index.apply_metadata(cell_id, metadata);
                self.corpus_synonym_store
                    .apply_record(cell_id, corpus_synonym_record);
                self.graph_index_store.apply_record(cell_id, graph_record);
                self.live_search_store
                    .apply_record(cell_id, live_search_record);
                self.search_context_store
                    .apply_record(cell_id, search_context_record);
                self.feedback_index.apply_record(cell_id, feedback_record);
                self.session_index.apply_record(cell_id, session_record);
                self.temporal_fact_store
                    .apply_record(cell_id, temporal_record);
                self.tool_index.apply_record(cell_id, tool_record);
                Ok(())
            }
            DbOperation::PatchCell { cell_id, payload } => {
                let descriptor =
                    descriptor.unwrap_or_else(|| CellDescriptor::from_payload_lossy(&payload));
                let metadata = CellMetadata::from_payload_with_descriptor(&payload, &descriptor);
                let corpus_synonym_record =
                    CorpusSynonymStore::record_from_payload(cell_id, &payload);
                let graph_record =
                    GraphIndexStore::record_from_payload(payload.clone(), &descriptor);
                let live_search_record =
                    LiveSearchStore::record_from_payload(payload.clone(), &descriptor);
                let search_context_record =
                    SearchContextStore::record_from_payload(payload.clone(), &descriptor);
                let feedback_record = FeedbackIndex::record_from_payload(&payload);
                let session_record =
                    SessionIndex::record_from_payload(cell_id, &payload, &descriptor);
                let temporal_record =
                    TemporalFactStore::record_from_payload(cell_id, &payload, &descriptor);
                let tool_record =
                    ToolIndex::record_from_payload(cell_id, seq, &payload, &descriptor);
                self.memtable
                    .patch_cell_with_descriptor(cell_id, seq, payload, descriptor)
                    .map_err(EngineError::from)?;
                self.aql_delta_index.apply_metadata(cell_id, metadata);
                self.corpus_synonym_store
                    .apply_record(cell_id, corpus_synonym_record);
                self.graph_index_store.apply_record(cell_id, graph_record);
                self.live_search_store
                    .apply_record(cell_id, live_search_record);
                self.search_context_store
                    .apply_record(cell_id, search_context_record);
                self.feedback_index.apply_record(cell_id, feedback_record);
                self.session_index.apply_record(cell_id, session_record);
                self.temporal_fact_store
                    .apply_record(cell_id, temporal_record);
                self.tool_index.apply_record(cell_id, tool_record);
                Ok(())
            }
            DbOperation::TombstoneCell { cell_id } => {
                self.memtable.record_tombstone(cell_id, seq);
                self.aql_delta_index.apply_tombstone(cell_id);
                self.corpus_synonym_store.apply_tombstone(cell_id);
                self.graph_index_store.apply_tombstone(cell_id);
                self.live_search_store.apply_tombstone(cell_id);
                self.search_context_store.apply_tombstone(cell_id);
                self.feedback_index.apply_tombstone(cell_id);
                self.session_index.apply_tombstone(cell_id);
                self.temporal_fact_store.apply_tombstone(cell_id);
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

    fn validate_write_batch(&self, batch: &WriteBatch) -> EngineResult<()> {
        let mut visible: BTreeSet<CellId> = self
            .memtable
            .live_cell_ids(self.read_txn())
            .into_iter()
            .collect();
        for operation in batch.operations() {
            match operation {
                WriteBatchOperation::PutCell { cell_id, .. } => {
                    visible.insert(*cell_id);
                }
                WriteBatchOperation::PatchCell { cell_id, .. } => {
                    if !visible.contains(cell_id) {
                        return Err(cortex_core::CoreError::CellNotFound(*cell_id).into());
                    }
                    visible.insert(*cell_id);
                }
                WriteBatchOperation::TombstoneCell { cell_id } => {
                    if !visible.remove(cell_id) {
                        return Err(cortex_core::CoreError::CellNotFound(*cell_id).into());
                    }
                }
            }
        }
        Ok(())
    }
}
