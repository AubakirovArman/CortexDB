use std::collections::{BTreeMap, BTreeSet};

use cortex_core::memtable::{MemTable, ReadTxn};
use cortex_core::{CellId, CommitSeq};

use super::metadata::CellMetadata;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct AqlDeltaIndex {
    changed_cell_ids: BTreeSet<CellId>,
    live_metadata: BTreeMap<CellId, CellMetadata>,
}

impl AqlDeltaIndex {
    pub(crate) fn from_memtable_after(memtable: &MemTable, txn: ReadTxn, seq: CommitSeq) -> Self {
        let changed_cell_ids = memtable
            .changed_cell_ids_after(seq)
            .into_iter()
            .collect::<BTreeSet<_>>();
        let live_metadata = changed_cell_ids
            .iter()
            .filter_map(|cell_id| {
                let version = memtable.read(txn, *cell_id)?;
                Some((
                    *cell_id,
                    CellMetadata::from_payload_with_descriptor(
                        &version.payload,
                        &version.descriptor,
                    ),
                ))
            })
            .collect();
        Self {
            changed_cell_ids,
            live_metadata,
        }
    }

    pub(crate) fn apply_metadata(&mut self, cell_id: CellId, metadata: CellMetadata) {
        self.changed_cell_ids.insert(cell_id);
        self.live_metadata.insert(cell_id, metadata);
    }

    pub(crate) fn apply_tombstone(&mut self, cell_id: CellId) {
        self.changed_cell_ids.insert(cell_id);
        self.live_metadata.remove(&cell_id);
    }

    pub(crate) fn clear(&mut self) {
        self.changed_cell_ids.clear();
        self.live_metadata.clear();
    }

    pub(crate) fn changed_cell_ids(&self) -> &BTreeSet<CellId> {
        &self.changed_cell_ids
    }

    pub(crate) fn live_cells(&self) -> impl Iterator<Item = (CellId, &CellMetadata)> {
        self.live_metadata
            .iter()
            .map(|(cell_id, metadata)| (*cell_id, metadata))
    }
}

#[cfg(test)]
mod tests {
    use cortex_core::CellDescriptor;
    use cortex_core::{CellId, CommitSeq};

    use super::*;

    #[test]
    fn delta_tracks_live_updates_and_tombstones() {
        let mut delta = AqlDeltaIndex::default();
        let descriptor = CellDescriptor::from_payload_lossy(
            b"scope=project:alpha\nstatus=ready\ntype=fact\n\nbody",
        );

        delta.apply_metadata(
            CellId(7),
            CellMetadata::from_payload_with_descriptor(
                b"scope=project:alpha\nstatus=ready\ntype=fact\n\nbody",
                &descriptor,
            ),
        );
        assert!(delta.changed_cell_ids().contains(&CellId(7)));
        assert_eq!(delta.live_cells().count(), 1);

        delta.apply_tombstone(CellId(7));
        assert!(delta.changed_cell_ids().contains(&CellId(7)));
        assert_eq!(delta.live_cells().count(), 0);

        delta.clear();
        assert!(delta.changed_cell_ids().is_empty());
    }

    #[test]
    fn delta_builds_once_from_memtable_changes_after_checkpoint() {
        let mut memtable = MemTable::default();
        memtable.put_cell(
            CellId(1),
            CommitSeq(1),
            b"scope=project:old\nstatus=ready\n\nold".to_vec(),
        );
        memtable
            .patch_cell(
                CellId(1),
                CommitSeq(3),
                b"scope=project:new\nstatus=ready\n\nnew".to_vec(),
            )
            .unwrap();
        memtable.record_tombstone(CellId(2), CommitSeq(4));

        let delta =
            AqlDeltaIndex::from_memtable_after(&memtable, ReadTxn::at(CommitSeq(4)), CommitSeq(2));

        assert_eq!(
            delta.changed_cell_ids().iter().copied().collect::<Vec<_>>(),
            vec![CellId(1), CellId(2)]
        );
        assert_eq!(delta.live_cells().count(), 1);
    }
}
