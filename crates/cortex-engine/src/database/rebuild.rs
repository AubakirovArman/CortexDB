use super::Database;
use crate::options::PayloadResidency;
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
        let mut temporal_store = TemporalFactStore::default();
        for version in self.memtable.visible_iter(txn) {
            if let Ok(payload) = self.payload_for_version_uncached(version) {
                conflict_store.apply_record(version.cell_id, &payload, &version.descriptor);
                temporal_store.apply_record(
                    version.cell_id,
                    TemporalFactStore::record_from_payload(
                        version.cell_id,
                        &payload,
                        &version.descriptor,
                    ),
                );
            }
        }
        self.conflict_index_store = conflict_store;
        self.temporal_fact_store = temporal_store;
    }
}
