use cortex_core::memtable::{MemTable, ReadTxn};
use cortex_core::{CellDescriptor, CellId, CommitSeq};

use crate::feedback::FeedbackIndex;
use crate::graph::GraphIndexStore;
use crate::memory::MemoryLifecycleStore;
use crate::options::PayloadResidency;
use crate::retrieval_quality::TemporalValidityStore;
use crate::search::{CorpusSynonymStore, LiveSearchStore, SearchContextStore};
use crate::session::SessionIndex;
use crate::tool_registry::ToolIndex;
use crate::verification::numeric::fact_claim::FactClaimStore;
use crate::verification::{ConflictIndexStore, TemporalFactStore};

mod derived_apply;

#[derive(Default)]
pub(super) struct DerivedStores {
    pub(super) corpus_synonym_store: CorpusSynonymStore,
    pub(super) feedback_index: FeedbackIndex,
    pub(super) graph_index_store: GraphIndexStore,
    pub(super) live_search_store: LiveSearchStore,
    pub(super) search_context_store: SearchContextStore,
    pub(super) session_index: SessionIndex,
    pub(super) memory_lifecycle_store: MemoryLifecycleStore,
    pub(super) fact_claim_store: FactClaimStore,
    pub(super) conflict_index_store: ConflictIndexStore,
    pub(super) temporal_fact_store: TemporalFactStore,
    pub(super) temporal_validity_store: TemporalValidityStore,
    pub(super) tool_index: ToolIndex,
}

impl DerivedStores {
    pub(super) fn from_memtable_for_residency(
        memtable: &MemTable,
        txn: ReadTxn,
        residency: PayloadResidency,
    ) -> Self {
        if residency == PayloadResidency::Lazy {
            return Self::from_resident_payloads(memtable, txn);
        }
        Self::from_memtable(memtable, txn)
    }

    pub(super) fn from_memtable(memtable: &MemTable, txn: ReadTxn) -> Self {
        Self {
            corpus_synonym_store: CorpusSynonymStore::from_memtable(memtable, txn),
            feedback_index: FeedbackIndex::from_memtable(memtable, txn),
            graph_index_store: GraphIndexStore::from_memtable(memtable, txn),
            live_search_store: LiveSearchStore::from_memtable(memtable, txn),
            search_context_store: SearchContextStore::from_memtable(memtable, txn),
            session_index: SessionIndex::from_memtable(memtable, txn),
            memory_lifecycle_store: MemoryLifecycleStore::from_memtable(memtable, txn),
            fact_claim_store: FactClaimStore::from_memtable(memtable, txn),
            conflict_index_store: ConflictIndexStore::from_memtable(memtable, txn),
            temporal_fact_store: TemporalFactStore::from_memtable(memtable, txn),
            temporal_validity_store: TemporalValidityStore::from_memtable(memtable, txn),
            tool_index: ToolIndex::from_memtable(memtable, txn),
        }
    }

    fn from_resident_payloads(memtable: &MemTable, txn: ReadTxn) -> Self {
        let mut stores = Self {
            temporal_validity_store: TemporalValidityStore::from_memtable(memtable, txn),
            memory_lifecycle_store: MemoryLifecycleStore::from_memtable(memtable, txn),
            session_index: SessionIndex::from_memtable(memtable, txn),
            ..Self::default()
        };
        for version in memtable
            .visible_iter(txn)
            .filter(|version| version.is_payload_resident())
        {
            stores.apply_record(
                version.cell_id,
                version.created_seq,
                &version.payload,
                &version.descriptor,
            );
        }
        stores
    }

    fn apply_record(
        &mut self,
        cell_id: CellId,
        seq: CommitSeq,
        payload: &[u8],
        descriptor: &CellDescriptor,
    ) {
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
}
