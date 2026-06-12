use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet};

use cortex_aql::AgentView;
use cortex_core::CellId;

use crate::database::Database;
use crate::query::scope_id;

use super::super::{classify_search_query_intent, SearchQuery, SearchQueryIntent};
use super::context::{
    high_level_anchor_score, is_search_parent_context_metadata, project_context_score,
    search_parent_lookup_keys,
};
use super::{metadata_for_version, DatabaseSearchResult};

impl Database {
    pub(crate) fn expand_search_parent_context(
        &self,
        results: Vec<DatabaseSearchResult>,
        view: &AgentView,
        limit: usize,
    ) -> Vec<DatabaseSearchResult> {
        if results.is_empty() || limit == 0 {
            return Vec::new();
        }
        if results.len() >= limit {
            return results.into_iter().take(limit).collect();
        }
        let parents = self.search_parent_context_candidates(view);
        if parents.is_empty() {
            return results.into_iter().take(limit).collect();
        }

        let mut expanded = Vec::with_capacity(limit);
        let mut emitted = BTreeSet::<CellId>::new();
        for result in results {
            if emitted.insert(result.cell_id) {
                expanded.push(result.clone());
            }
            if expanded.len() >= limit {
                break;
            }
            for key in search_parent_lookup_keys(&result.metadata) {
                let Some(parent) = parents.get(&key) else {
                    continue;
                };
                if parent.cell_id == result.cell_id || !emitted.insert(parent.cell_id) {
                    continue;
                }
                let mut parent = parent.clone();
                parent.score = parent.score.max(result.score.saturating_sub(1));
                expanded.push(parent);
                break;
            }
            if expanded.len() >= limit {
                break;
            }
        }
        expanded
    }

    pub(crate) fn expand_high_level_anchor_context(
        &self,
        results: Vec<DatabaseSearchResult>,
        view: &AgentView,
        query: SearchQuery<'_>,
    ) -> Vec<DatabaseSearchResult> {
        if query.limit == 0
            || classify_search_query_intent(query.text) != SearchQueryIntent::HighLevel
        {
            return results.into_iter().take(query.limit).collect();
        }

        let anchors = self.high_level_anchor_candidates(view);
        if anchors.is_empty() {
            return results.into_iter().take(query.limit).collect();
        }

        let mut expanded = Vec::with_capacity(query.limit);
        let mut emitted = BTreeSet::<CellId>::new();
        for anchor in anchors {
            if emitted.insert(anchor.cell_id) {
                expanded.push(anchor);
            }
            if expanded.len() >= query.limit {
                return expanded;
            }
        }
        for result in results {
            if emitted.insert(result.cell_id) {
                expanded.push(result);
            }
            if expanded.len() >= query.limit {
                break;
            }
        }
        expanded
    }

    pub(crate) fn expand_project_related_context(
        &self,
        results: Vec<DatabaseSearchResult>,
        view: &AgentView,
        query: SearchQuery<'_>,
    ) -> Vec<DatabaseSearchResult> {
        if query.limit == 0
            || classify_search_query_intent(query.text) != SearchQueryIntent::ProjectRelated
        {
            return results.into_iter().take(query.limit).collect();
        }
        let projects = results
            .iter()
            .filter_map(|result| result.metadata.project.clone())
            .collect::<BTreeSet<_>>();
        if projects.is_empty() {
            return results.into_iter().take(query.limit).collect();
        }

        let project_candidates = self.project_context_candidates(view, &projects);
        let mut expanded = Vec::with_capacity(query.limit);
        let mut emitted = BTreeSet::<CellId>::new();
        for result in results {
            if emitted.insert(result.cell_id) {
                expanded.push(result);
            }
            if expanded.len() >= query.limit {
                return expanded;
            }
        }
        for candidate in project_candidates {
            if emitted.insert(candidate.cell_id) {
                expanded.push(candidate);
            }
            if expanded.len() >= query.limit {
                break;
            }
        }
        expanded
    }

    fn project_context_candidates(
        &self,
        view: &AgentView,
        projects: &BTreeSet<String>,
    ) -> Vec<DatabaseSearchResult> {
        let mut candidates = self
            .snapshot_versions()
            .into_iter()
            .filter_map(|version| {
                let metadata = metadata_for_version(&version);
                if !view.can_read_scope(scope_id(&metadata.scope))
                    || !metadata
                        .project
                        .as_ref()
                        .is_some_and(|project| projects.contains(project))
                {
                    return None;
                }
                let score = project_context_score(&metadata);
                Some(DatabaseSearchResult {
                    cell_id: version.cell_id,
                    score,
                    lexical_score: score,
                    vector_score: 0,
                    metadata,
                    payload: version.payload,
                })
            })
            .collect::<Vec<_>>();
        candidates.sort_by_key(|result| (Reverse(result.score), result.cell_id.0));
        candidates
    }

    fn high_level_anchor_candidates(&self, view: &AgentView) -> Vec<DatabaseSearchResult> {
        let mut candidates = self
            .snapshot_versions()
            .into_iter()
            .filter_map(|version| {
                let metadata = metadata_for_version(&version);
                if !view.can_read_scope(scope_id(&metadata.scope)) {
                    return None;
                }
                let score = high_level_anchor_score(&metadata);
                (score > 0).then_some(DatabaseSearchResult {
                    cell_id: version.cell_id,
                    score,
                    lexical_score: score,
                    vector_score: 0,
                    metadata,
                    payload: version.payload,
                })
            })
            .collect::<Vec<_>>();
        candidates.sort_by_key(|result| (Reverse(result.score), result.cell_id.0));
        candidates
    }

    fn search_parent_context_candidates(
        &self,
        view: &AgentView,
    ) -> BTreeMap<String, DatabaseSearchResult> {
        let mut parents = BTreeMap::new();
        for version in self.snapshot_versions() {
            let metadata = metadata_for_version(&version);
            if !view.can_read_scope(scope_id(&metadata.scope))
                || !is_search_parent_context_metadata(&metadata)
            {
                continue;
            }
            let result = DatabaseSearchResult {
                cell_id: version.cell_id,
                score: 0,
                lexical_score: 0,
                vector_score: 0,
                metadata: metadata.clone(),
                payload: version.payload,
            };
            if let Some(chunk_id) = &metadata.chunk_id {
                parents.entry(chunk_id.clone()).or_insert(result.clone());
            }
            if let Some(document_id) = &metadata.document_id {
                parents.entry(document_id.clone()).or_insert(result);
            }
        }
        parents
    }
}
