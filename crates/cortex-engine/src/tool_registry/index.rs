use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet};

use cortex_aql::AgentView;
use cortex_core::memtable::{MemTable, ReadTxn};
use cortex_core::{CellDescriptor, CellId, CommitSeq};

use crate::plan::PolicyRewrite;
use crate::query::scope_id;
use crate::search::tokenize;

use super::{RegisteredTool, ToolDescriptor, ToolRecommendation};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct ToolIndex {
    tools: BTreeMap<CellId, RegisteredTool>,
    term_to_tools: BTreeMap<String, BTreeSet<CellId>>,
    tool_terms: BTreeMap<CellId, BTreeSet<String>>,
}

impl ToolIndex {
    pub(crate) fn from_memtable(memtable: &MemTable, txn: ReadTxn) -> Self {
        let mut index = Self::default();
        for version in memtable.visible_iter(txn) {
            let Ok(descriptor) = ToolDescriptor::from_version(version) else {
                continue;
            };
            index.insert_tool(RegisteredTool {
                cell_id: version.cell_id,
                commit_seq: version.created_seq,
                descriptor,
            });
        }
        index
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
        self.remove_tool(cell_id);
        if let Some(tool) = tool {
            self.insert_tool(RegisteredTool { cell_id, ..tool });
        }
    }

    pub(crate) fn apply_tombstone(&mut self, cell_id: CellId) {
        self.remove_tool(cell_id);
    }

    pub(crate) fn list_tools(&self, view: &AgentView) -> Vec<RegisteredTool> {
        self.tools
            .values()
            .filter(|tool| PolicyRewrite::allows_scope(view, scope_id(&tool.descriptor.scope)))
            .cloned()
            .collect()
    }

    pub(crate) fn recommend_tools_for_task(
        &self,
        view: &AgentView,
        task: &str,
        limit: usize,
    ) -> Vec<ToolRecommendation> {
        if limit == 0 {
            return Vec::new();
        }
        let mut matches: BTreeMap<CellId, BTreeSet<String>> = BTreeMap::new();
        for term in tokenize(task) {
            let Some(cell_ids) = self.term_to_tools.get(&term) else {
                continue;
            };
            for cell_id in cell_ids {
                matches.entry(*cell_id).or_default().insert(term.clone());
            }
        }
        let mut recommendations = matches
            .into_iter()
            .filter_map(|(cell_id, matched)| {
                let tool = self.tools.get(&cell_id)?;
                if !PolicyRewrite::allows_scope(view, scope_id(&tool.descriptor.scope)) {
                    return None;
                }
                let matched_terms = matched.into_iter().collect::<Vec<_>>();
                let score = u32::try_from(matched_terms.len()).unwrap_or(u32::MAX);
                let why_selected = format!(
                    "selected because task terms matched tool metadata: {}",
                    matched_terms.join(", ")
                );
                Some(ToolRecommendation {
                    tool: tool.clone(),
                    matched_terms,
                    score,
                    why_selected,
                })
            })
            .collect::<Vec<_>>();
        recommendations.sort_by_key(|recommendation| {
            (
                Reverse(recommendation.score),
                recommendation.tool.cell_id,
                recommendation.tool.descriptor.name.clone(),
            )
        });
        recommendations.truncate(limit);
        recommendations
    }

    fn insert_tool(&mut self, tool: RegisteredTool) {
        let cell_id = tool.cell_id;
        self.remove_tool(cell_id);
        let terms = terms_for_descriptor(&tool.descriptor);
        for term in &terms {
            self.term_to_tools
                .entry(term.clone())
                .or_default()
                .insert(cell_id);
        }
        self.tool_terms.insert(cell_id, terms);
        self.tools.insert(cell_id, tool);
    }

    fn remove_tool(&mut self, cell_id: CellId) {
        if let Some(terms) = self.tool_terms.remove(&cell_id) {
            for term in terms {
                let should_remove = if let Some(cell_ids) = self.term_to_tools.get_mut(&term) {
                    cell_ids.remove(&cell_id);
                    cell_ids.is_empty()
                } else {
                    false
                };
                if should_remove {
                    self.term_to_tools.remove(&term);
                }
            }
        }
        self.tools.remove(&cell_id);
    }
}

fn terms_for_descriptor(descriptor: &ToolDescriptor) -> BTreeSet<String> {
    let descriptor_text = format!(
        "{} {} {} {}",
        descriptor.name,
        descriptor.description,
        descriptor.input_schema.as_deref().unwrap_or(""),
        descriptor.output_schema.as_deref().unwrap_or("")
    );
    tokenize(&descriptor_text).into_iter().collect()
}
