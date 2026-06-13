use std::collections::BTreeMap;

use cortex_aql::AgentView;
use cortex_core::memtable::{MemTable, ReadTxn};
use cortex_core::{CellDescriptor, CellId};

use crate::database::RetrievedCell;
use crate::query::{scope_id, CellMetadata};

use super::payload::parse_session_cell;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct SessionIndex {
    cells: BTreeMap<CellId, SessionIndexCell>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SessionIndexCell {
    session_id: String,
    expires_at_unix_seconds: u64,
    retrieved: RetrievedCell,
}

impl SessionIndex {
    pub(crate) fn from_memtable(memtable: &MemTable, txn: ReadTxn) -> Self {
        let cells = memtable
            .visible_iter(txn)
            .filter_map(|version| {
                Self::record_from_payload(version.cell_id, &version.payload, &version.descriptor)
            })
            .collect();
        Self { cells }
    }

    pub(crate) fn record_from_payload(
        cell_id: CellId,
        payload: &[u8],
        descriptor: &CellDescriptor,
    ) -> Option<(CellId, SessionIndexCell)> {
        let session_metadata = parse_session_cell(payload)?;
        let expires_at_unix_seconds = session_metadata.expires_at_unix_seconds();
        let retrieved = RetrievedCell {
            cell_id,
            payload: payload.to_vec(),
            descriptor: descriptor.clone(),
        };
        Some((
            cell_id,
            SessionIndexCell {
                session_id: session_metadata.session_id,
                expires_at_unix_seconds,
                retrieved,
            },
        ))
    }

    pub(crate) fn apply_record(
        &mut self,
        cell_id: CellId,
        record: Option<(CellId, SessionIndexCell)>,
    ) {
        if let Some((_, record)) = record {
            self.cells.insert(cell_id, record);
        } else {
            self.cells.remove(&cell_id);
        }
    }

    pub(crate) fn apply_tombstone(&mut self, cell_id: CellId) {
        self.cells.remove(&cell_id);
    }

    pub(crate) fn retrieve(
        &self,
        session_id: &str,
        view: &AgentView,
        now_unix_seconds: u64,
    ) -> Vec<RetrievedCell> {
        self.cells
            .values()
            .filter(|cell| {
                let descriptor_metadata = CellMetadata::from_payload_with_descriptor(
                    &cell.retrieved.payload,
                    &cell.retrieved.descriptor,
                );
                cell.session_id == session_id
                    && now_unix_seconds < cell.expires_at_unix_seconds
                    && view.can_read_scope(scope_id(&descriptor_metadata.scope))
            })
            .map(|cell| cell.retrieved.clone())
            .collect()
    }

    pub(crate) fn retrieve_from_payload(
        cell_id: CellId,
        payload: &[u8],
        descriptor: &CellDescriptor,
        session_id: &str,
        view: &AgentView,
        now_unix_seconds: u64,
    ) -> Option<RetrievedCell> {
        let (_, cell) = Self::record_from_payload(cell_id, payload, descriptor)?;
        let descriptor_metadata =
            CellMetadata::from_payload_with_descriptor(&cell.retrieved.payload, descriptor);
        (cell.session_id == session_id
            && now_unix_seconds < cell.expires_at_unix_seconds
            && view.can_read_scope(scope_id(&descriptor_metadata.scope)))
        .then_some(cell.retrieved)
    }
}
