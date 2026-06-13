use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet};

use cortex_aql::AgentView;
use cortex_core::memtable::{MemTable, ReadTxn};
use cortex_core::{CellDescriptor, CellId};

use crate::query::{scope_id, CellMetadata};

use super::context::{
    high_level_anchor_score, is_search_parent_context_metadata, project_context_score,
};
use super::types::DatabaseSearchResult;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct SearchContextStore {
    records: BTreeMap<CellId, SearchContextRecord>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SearchContextRecord {
    payload: Vec<u8>,
    metadata: CellMetadata,
}

impl SearchContextStore {
    pub(crate) fn from_memtable(memtable: &MemTable, txn: ReadTxn) -> Self {
        let records = memtable
            .visible_iter(txn)
            .filter_map(|version| {
                Self::record_from_payload(version.payload.clone(), &version.descriptor)
                    .map(|record| (version.cell_id, record))
            })
            .collect();
        Self { records }
    }

    pub(crate) fn record_from_payload(
        payload: Vec<u8>,
        descriptor: &CellDescriptor,
    ) -> Option<SearchContextRecord> {
        let metadata = CellMetadata::from_payload_with_descriptor(&payload, descriptor);
        is_search_context_relevant(&metadata).then_some(SearchContextRecord { payload, metadata })
    }

    pub(crate) fn apply_record(&mut self, cell_id: CellId, record: Option<SearchContextRecord>) {
        if let Some(record) = record {
            self.records.insert(cell_id, record);
        } else {
            self.records.remove(&cell_id);
        }
    }

    pub(crate) fn apply_tombstone(&mut self, cell_id: CellId) {
        self.records.remove(&cell_id);
    }

    pub(crate) fn project_context_candidates(
        &self,
        view: &AgentView,
        projects: &BTreeSet<String>,
    ) -> Vec<DatabaseSearchResult> {
        let mut candidates = self
            .records
            .iter()
            .filter_map(|(cell_id, record)| {
                if !view.can_read_scope(scope_id(&record.metadata.scope))
                    || !record
                        .metadata
                        .project
                        .as_ref()
                        .is_some_and(|project| projects.contains(project))
                {
                    return None;
                }
                let score = project_context_score(&record.metadata);
                Some(search_result(
                    *cell_id,
                    score,
                    record.metadata.clone(),
                    record.payload.clone(),
                ))
            })
            .collect::<Vec<_>>();
        candidates.sort_by_key(|result| (Reverse(result.score), result.cell_id.0));
        candidates
    }

    pub(crate) fn high_level_anchor_candidates(
        &self,
        view: &AgentView,
    ) -> Vec<DatabaseSearchResult> {
        let mut candidates = self
            .records
            .iter()
            .filter_map(|(cell_id, record)| {
                if !view.can_read_scope(scope_id(&record.metadata.scope)) {
                    return None;
                }
                let score = high_level_anchor_score(&record.metadata);
                (score > 0).then(|| {
                    search_result(
                        *cell_id,
                        score,
                        record.metadata.clone(),
                        record.payload.clone(),
                    )
                })
            })
            .collect::<Vec<_>>();
        candidates.sort_by_key(|result| (Reverse(result.score), result.cell_id.0));
        candidates
    }

    pub(crate) fn search_parent_context_candidates(
        &self,
        view: &AgentView,
    ) -> BTreeMap<String, DatabaseSearchResult> {
        let mut parents = BTreeMap::new();
        for (cell_id, record) in &self.records {
            if !view.can_read_scope(scope_id(&record.metadata.scope))
                || !is_search_parent_context_metadata(&record.metadata)
            {
                continue;
            }
            let result =
                search_result(*cell_id, 0, record.metadata.clone(), record.payload.clone());
            if let Some(chunk_id) = &record.metadata.chunk_id {
                parents.entry(chunk_id.clone()).or_insert(result.clone());
            }
            if let Some(document_id) = &record.metadata.document_id {
                parents.entry(document_id.clone()).or_insert(result);
            }
        }
        parents
    }
}

fn is_search_context_relevant(metadata: &CellMetadata) -> bool {
    metadata.project.is_some()
        || high_level_anchor_score(metadata) > 0
        || is_search_parent_context_metadata(metadata)
}

fn search_result(
    cell_id: CellId,
    score: u64,
    metadata: CellMetadata,
    payload: Vec<u8>,
) -> DatabaseSearchResult {
    DatabaseSearchResult {
        cell_id,
        score,
        lexical_score: score,
        vector_score: 0,
        metadata,
        payload,
    }
}
