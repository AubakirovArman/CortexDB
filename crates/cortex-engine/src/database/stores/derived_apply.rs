use cortex_core::{CellDescriptor, CellId, CommitSeq};

#[cfg(feature = "experimental-replication")]
use super::DerivedStores;
use crate::query::CellMetadata;

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
        self.derived_stores
            .apply_record(cell_id, seq, payload, descriptor);
    }

    pub(in crate::database) fn apply_derived_tombstone(&mut self, cell_id: CellId) {
        self.aql_delta_index.apply_tombstone(cell_id);
        self.derived_stores.apply_tombstone(cell_id);
    }

    #[cfg(feature = "experimental-replication")]
    pub(crate) fn rebuild_derived_stores_from_memtable(&mut self) {
        let stores = DerivedStores::from_memtable_for_residency(
            &self.memtable,
            self.read_txn(),
            self.payload_residency,
        );
        self.derived_stores = stores;
        self.rebuild_lazy_derived_stores_for_residency(self.payload_residency);
    }
}
