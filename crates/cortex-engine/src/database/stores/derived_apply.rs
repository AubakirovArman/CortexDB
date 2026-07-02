use cortex_core::{CellDescriptor, CellId, CommitSeq};

use super::DerivedStores;
use crate::feedback::FeedbackIndex;
use crate::graph::GraphIndexStore;
use crate::query::CellMetadata;
use crate::search::{CorpusSynonymStore, LiveSearchStore, SearchContextStore};
use crate::session::SessionIndex;
use crate::tool_registry::ToolIndex;
use crate::verification::numeric::fact_claim::FactClaimStore;
use crate::verification::TemporalFactStore;

impl super::super::Database {
    pub(in crate::database) fn apply_derived_cell_record(
        &mut self,
        cell_id: CellId,
        seq: CommitSeq,
        payload: &[u8],
        descriptor: &CellDescriptor,
    ) {
        let metadata = CellMetadata::from_payload_with_descriptor(payload, descriptor);
        self.aql_delta_index.apply_metadata(cell_id, metadata);
        self.corpus_synonym_store.apply_record(
            cell_id,
            CorpusSynonymStore::record_from_payload(cell_id, payload),
        );
        self.feedback_index
            .apply_record(cell_id, FeedbackIndex::record_from_payload(payload));
        self.graph_index_store.apply_record(
            cell_id,
            GraphIndexStore::record_from_payload(payload.to_vec(), descriptor),
        );
        self.live_search_store.apply_record(
            cell_id,
            LiveSearchStore::record_from_payload(payload.to_vec(), descriptor),
        );
        self.search_context_store.apply_record(
            cell_id,
            SearchContextStore::record_from_payload(payload.to_vec(), descriptor),
        );
        self.session_index.apply_record(
            cell_id,
            SessionIndex::record_from_payload(cell_id, payload, descriptor),
        );
        self.memory_lifecycle_store
            .apply_descriptor(cell_id, descriptor);
        self.fact_claim_store.apply_records(
            cell_id,
            FactClaimStore::records_from_payload(cell_id, payload, descriptor),
        );
        self.conflict_index_store
            .apply_record(cell_id, payload, descriptor);
        self.temporal_fact_store.apply_record(
            cell_id,
            TemporalFactStore::record_from_payload(cell_id, payload, descriptor),
        );
        self.temporal_validity_store
            .apply_descriptor(cell_id, descriptor);
        self.tool_index.apply_record(
            cell_id,
            ToolIndex::record_from_payload(cell_id, seq, payload, descriptor),
        );
    }

    pub(in crate::database) fn apply_derived_tombstone(&mut self, cell_id: CellId) {
        self.aql_delta_index.apply_tombstone(cell_id);
        self.corpus_synonym_store.apply_tombstone(cell_id);
        self.feedback_index.apply_tombstone(cell_id);
        self.graph_index_store.apply_tombstone(cell_id);
        self.live_search_store.apply_tombstone(cell_id);
        self.search_context_store.apply_tombstone(cell_id);
        self.session_index.apply_tombstone(cell_id);
        self.memory_lifecycle_store.apply_tombstone(cell_id);
        self.fact_claim_store.apply_tombstone(cell_id);
        self.conflict_index_store.apply_tombstone(cell_id);
        self.temporal_fact_store.apply_tombstone(cell_id);
        self.temporal_validity_store.apply_tombstone(cell_id);
        self.tool_index.apply_tombstone(cell_id);
    }

    pub(crate) fn rebuild_derived_stores_from_memtable(&mut self) {
        let stores = DerivedStores::from_memtable_for_residency(
            &self.memtable,
            self.read_txn(),
            self.payload_residency,
        );
        self.corpus_synonym_store = stores.corpus_synonym_store;
        self.feedback_index = stores.feedback_index;
        self.graph_index_store = stores.graph_index_store;
        self.live_search_store = stores.live_search_store;
        self.search_context_store = stores.search_context_store;
        self.session_index = stores.session_index;
        self.memory_lifecycle_store = stores.memory_lifecycle_store;
        self.fact_claim_store = stores.fact_claim_store;
        self.conflict_index_store = stores.conflict_index_store;
        self.temporal_fact_store = stores.temporal_fact_store;
        self.temporal_validity_store = stores.temporal_validity_store;
        self.tool_index = stores.tool_index;
        self.rebuild_lazy_derived_stores_for_residency(self.payload_residency);
    }
}
