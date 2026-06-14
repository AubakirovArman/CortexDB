use super::Database;
use crate::graph::GraphIndexStore;
use crate::options::PayloadResidency;
use crate::tool_registry::ToolIndex;
use crate::verification::{ConflictIndexStore, TemporalFactStore};

impl Database {
    pub(crate) fn rebuild_lazy_derived_stores_for_residency(
        &mut self,
        payload_residency: PayloadResidency,
    ) {
        if payload_residency == PayloadResidency::Lazy {
            self.rebuild_lazy_derived_stores_from_visible_payloads();
        }
    }

    pub(crate) fn rebuild_lazy_derived_stores_from_visible_payloads(&mut self) {
        let pin = self.pin_read_txn();
        let txn = pin.read_txn();
        let mut conflict_store = ConflictIndexStore::default();
        let mut graph_index_store = GraphIndexStore::default();
        let mut temporal_store = TemporalFactStore::default();
        let mut tool_index = ToolIndex::default();
        for version in self.memtable.visible_iter(txn) {
            if let Ok(payload) = self.payload_for_version_uncached(version) {
                conflict_store.apply_record(version.cell_id, &payload, &version.descriptor);
                graph_index_store.apply_record(
                    version.cell_id,
                    GraphIndexStore::record_from_payload(payload.clone(), &version.descriptor),
                );
                temporal_store.apply_record(
                    version.cell_id,
                    TemporalFactStore::record_from_payload(
                        version.cell_id,
                        &payload,
                        &version.descriptor,
                    ),
                );
                tool_index.apply_record(
                    version.cell_id,
                    ToolIndex::record_from_payload(
                        version.cell_id,
                        version.created_seq,
                        &payload,
                        &version.descriptor,
                    ),
                );
            }
        }
        self.conflict_index_store = conflict_store;
        self.graph_index_store = graph_index_store;
        self.temporal_fact_store = temporal_store;
        self.tool_index = tool_index;
    }
}
