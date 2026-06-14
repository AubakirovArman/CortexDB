use std::collections::BTreeMap;

use cortex_core::memtable::{MemTable, ReadTxn};
use cortex_core::{CellDescriptor, CellId};

use crate::query::metadata::CellMetadata;

use super::types::{KnowledgeGraphIndex, ToolCell};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct GraphIndexStore {
    records: BTreeMap<CellId, GraphIndexRecord>,
    index: KnowledgeGraphIndex,
    tools: BTreeMap<CellId, ToolCell>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct GraphIndexRecord {
    payload: Vec<u8>,
    metadata: CellMetadata,
}

impl GraphIndexStore {
    pub(crate) fn from_memtable(memtable: &MemTable, txn: ReadTxn) -> Self {
        Self::from_records(memtable.visible_iter(txn).filter_map(|version| {
            Self::record_from_payload(version.payload.clone(), &version.descriptor)
                .map(|record| (version.cell_id, record))
        }))
    }

    pub(crate) fn record_from_payload(
        payload: Vec<u8>,
        descriptor: &CellDescriptor,
    ) -> Option<GraphIndexRecord> {
        let metadata = CellMetadata::from_payload_with_descriptor(&payload, descriptor);
        is_graph_relevant(&metadata).then_some(GraphIndexRecord { payload, metadata })
    }

    pub(crate) fn apply_record(&mut self, cell_id: CellId, record: Option<GraphIndexRecord>) {
        self.remove_record(cell_id);
        if let Some(record) = record {
            self.insert_record(cell_id, record);
        } else {
            self.records.remove(&cell_id);
        }
    }

    pub(crate) fn apply_tombstone(&mut self, cell_id: CellId) {
        self.remove_record(cell_id);
    }

    pub(crate) fn index(&self) -> KnowledgeGraphIndex {
        self.index.clone()
    }

    pub(crate) fn tool_cells(&self) -> Vec<ToolCell> {
        self.tools.values().cloned().collect()
    }

    pub(crate) fn from_records(
        records: impl IntoIterator<Item = (CellId, GraphIndexRecord)>,
    ) -> Self {
        let mut store = Self::default();
        for (cell_id, record) in records {
            store.insert_record(cell_id, record);
        }
        store
    }

    fn insert_record(&mut self, cell_id: CellId, record: GraphIndexRecord) {
        self.index
            .index_record(cell_id, record.payload.as_slice(), &record.metadata);
        if let Some((_, tool)) = tool_cell_from_record(cell_id, &record) {
            self.tools.insert(cell_id, tool);
        }
        self.records.insert(cell_id, record);
    }

    fn remove_record(&mut self, cell_id: CellId) {
        self.records.remove(&cell_id);
        self.index.remove_cell(cell_id);
        self.tools.remove(&cell_id);
    }
}

fn is_graph_relevant(metadata: &CellMetadata) -> bool {
    metadata.source_ref.is_some()
        || matches!(metadata.cell_type.as_str(), "entity" | "relation" | "tool")
}

fn tool_cell_from_record(cell_id: CellId, record: &GraphIndexRecord) -> Option<(CellId, ToolCell)> {
    if record.metadata.cell_type != "tool" {
        return None;
    }
    let name = record
        .metadata
        .body_text
        .lines()
        .find(|line| line.starts_with("name="))
        .and_then(|line| line.strip_prefix("name="))
        .map(ToOwned::to_owned);
    Some((
        cell_id,
        ToolCell {
            cell_id,
            name,
            description: record.metadata.body_text.clone(),
        },
    ))
}
