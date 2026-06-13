use std::collections::BTreeMap;

use cortex_aql::AgentView;
use cortex_core::memtable::{MemTable, ReadTxn};
use cortex_core::{CellDescriptor, CellId, CommitSeq};

use crate::query::scope_id;

use super::{RegisteredTool, ToolDescriptor};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct ToolIndex {
    tools: BTreeMap<CellId, RegisteredTool>,
}

impl ToolIndex {
    pub(crate) fn from_memtable(memtable: &MemTable, txn: ReadTxn) -> Self {
        let tools = memtable
            .visible_iter(txn)
            .filter_map(|version| {
                let descriptor = ToolDescriptor::from_version(version).ok()?;
                Some((
                    version.cell_id,
                    RegisteredTool {
                        cell_id: version.cell_id,
                        commit_seq: version.created_seq,
                        descriptor,
                    },
                ))
            })
            .collect();
        Self { tools }
    }

    pub(crate) fn record_from_payload(
        cell_id: CellId,
        commit_seq: CommitSeq,
        payload: &[u8],
        descriptor: &CellDescriptor,
    ) -> Option<RegisteredTool> {
        let descriptor = ToolDescriptor::from_payload_with_descriptor(payload, descriptor).ok()?;
        Some(RegisteredTool {
            cell_id,
            commit_seq,
            descriptor,
        })
    }

    pub(crate) fn apply_record(&mut self, cell_id: CellId, tool: Option<RegisteredTool>) {
        if let Some(tool) = tool {
            self.tools.insert(cell_id, tool);
        } else {
            self.tools.remove(&cell_id);
        }
    }

    pub(crate) fn apply_tombstone(&mut self, cell_id: CellId) {
        self.tools.remove(&cell_id);
    }

    pub(crate) fn list_tools(&self, view: &AgentView) -> Vec<RegisteredTool> {
        self.tools
            .values()
            .filter(|tool| view.can_read_scope(scope_id(&tool.descriptor.scope)))
            .cloned()
            .collect()
    }
}
