use std::collections::{BTreeMap, BTreeSet};

use cortex_aql::AgentView;
use cortex_core::memtable::{MemTable, ReadTxn};
use cortex_core::{CellDescriptor, CellId};

use crate::database::RetrievedCell;

mod record;

use record::SessionIndexCell;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct SessionIndex {
    cells: BTreeMap<CellId, SessionIndexCell>,
    cells_by_session: BTreeMap<String, BTreeSet<CellId>>,
}

impl SessionIndex {
    pub(crate) fn from_memtable(memtable: &MemTable, txn: ReadTxn) -> Self {
        let mut index = Self::default();
        for version in memtable.visible_iter(txn) {
            index.apply_record(
                version.cell_id,
                SessionIndexCell::from_version(
                    version.cell_id,
                    &version.payload,
                    &version.descriptor,
                    version.is_payload_resident(),
                )
                .map(|record| (version.cell_id, record)),
            );
        }
        index
    }

    pub(crate) fn record_from_payload(
        cell_id: CellId,
        payload: &[u8],
        descriptor: &CellDescriptor,
    ) -> Option<(CellId, SessionIndexCell)> {
        SessionIndexCell::from_version(cell_id, payload, descriptor, true)
            .map(|record| (cell_id, record))
    }

    pub(crate) fn apply_record(
        &mut self,
        cell_id: CellId,
        record: Option<(CellId, SessionIndexCell)>,
    ) {
        self.remove_record(cell_id);
        if let Some((_, record)) = record {
            self.cells_by_session
                .entry(record.session_id().to_owned())
                .or_default()
                .insert(cell_id);
            self.cells.insert(cell_id, record);
        }
    }

    pub(crate) fn apply_tombstone(&mut self, cell_id: CellId) {
        self.remove_record(cell_id);
    }

    pub(crate) fn retrieve<F>(
        &self,
        session_id: &str,
        view: &AgentView,
        now_unix_seconds: u64,
        mut load_payload: F,
    ) -> Vec<RetrievedCell>
    where
        F: FnMut(CellId, Option<&[u8]>, &CellDescriptor) -> Option<Vec<u8>>,
    {
        self.cells_by_session
            .get(session_id)
            .into_iter()
            .flat_map(|cell_ids| cell_ids.iter())
            .filter_map(|cell_id| {
                let cell = self.cells.get(cell_id)?;
                if !cell.is_visible_to(view, now_unix_seconds) {
                    return None;
                }
                let payload = load_payload(*cell_id, cell.resident_payload(), cell.descriptor())?;
                Some(RetrievedCell {
                    cell_id: *cell_id,
                    payload,
                    descriptor: cell.descriptor().clone(),
                    captured_access_decision: None,
                })
            })
            .collect()
    }

    fn remove_record(&mut self, cell_id: CellId) {
        let Some(record) = self.cells.remove(&cell_id) else {
            return;
        };
        if let Some(cell_ids) = self.cells_by_session.get_mut(record.session_id()) {
            cell_ids.remove(&cell_id);
            if cell_ids.is_empty() {
                self.cells_by_session.remove(record.session_id());
            }
        }
    }
}
