use std::collections::BTreeMap;

use cortex_aql::AgentView;
use cortex_core::CellId;

use crate::database::Database;
use crate::error::{EngineError, EngineResult};
use crate::query::{scope_id, CellMetadata};

use super::super::ann::AnnSearchPolicy;
use super::super::{SearchIndexes, SearchMode, SearchQuery, WeightedScoreReranker};
use super::ann_reports::snapshot_ann_report;
use super::diversity::select_diverse_results;
use super::ranking::{
    best_payload_vector_for_query, database_index_query, rerank_database_results,
};
use super::trace::trace_search;
use super::{metadata_for_version, DatabaseSearchOutcome, DatabaseSearchResult, SearchViewTrace};

impl Database {
    pub fn search_cells_with_report_with_policy(
        &self,
        query: SearchQuery<'_>,
        view: &AgentView,
        policy: Option<AnnSearchPolicy>,
    ) -> EngineResult<DatabaseSearchOutcome> {
        if let Some(results) = self.search_persisted_query(query, view, policy)? {
            return Ok(results);
        }
        trace_search(&format!(
            "snapshot search rebuild begin mode={:?} limit={}",
            query.mode, query.limit
        ));
        let mut indexes = SearchIndexes::default();
        let mut cells = BTreeMap::<u32, (CellId, Vec<u8>, CellMetadata)>::new();
        let mut traces = BTreeMap::<u32, SearchViewTrace>::new();
        let mut vector_candidates = 0usize;
        for (index, (version, metadata)) in self
            .snapshot_versions()
            .into_iter()
            .filter_map(|version| {
                let metadata = metadata_for_version(&version);
                view.can_read_scope(scope_id(&metadata.scope))
                    .then_some((version, metadata))
            })
            .enumerate()
        {
            let candidate =
                u32::try_from(index + 1).map_err(|_| EngineError::CandidateIdOverflow)?;
            indexes.add_field_terms(candidate, metadata.lexical_field_terms());
            if let Some(best) = best_payload_vector_for_query(&version.payload, query.vector) {
                traces.insert(
                    candidate,
                    SearchViewTrace {
                        cell_id: version.cell_id,
                        candidate_id: candidate,
                        vector_view: best.view_name,
                        vector_score: best.score,
                    },
                );
                indexes.add_vector(candidate, best.vector);
                vector_candidates += 1;
            }
            cells.insert(candidate, (version.cell_id, version.payload, metadata));
        }
        let expanded_query_text = self.corpus_synonym_expanded_query_text(query.text)?;
        let index_query_text = expanded_query_text.as_deref().unwrap_or(query.text);
        let index_query = database_index_query(SearchQuery {
            text: index_query_text,
            ..query
        });
        trace_search(&format!(
            "snapshot search indexed cells={} vector_candidates={}",
            cells.len(),
            vector_candidates
        ));
        let indexed_results = indexes
            .search(index_query)
            .into_iter()
            .filter_map(|result| {
                let candidate_id = result.cell_id;
                let (cell_id, payload, metadata) = cells.remove(&candidate_id)?;
                Some((
                    candidate_id,
                    DatabaseSearchResult {
                        cell_id,
                        score: result.score,
                        lexical_score: result.lexical_score,
                        vector_score: result.vector_score,
                        metadata,
                        payload,
                    },
                ))
            })
            .collect::<Vec<_>>();
        let traces_by_cell = indexed_results
            .iter()
            .filter_map(|(candidate_id, result)| {
                traces
                    .get(candidate_id)
                    .cloned()
                    .map(|trace| (result.cell_id, trace))
            })
            .collect::<BTreeMap<_, _>>();
        let mut raw_results = indexed_results
            .into_iter()
            .map(|(_, result)| result)
            .collect::<Vec<_>>();
        let mut diversity_diagnostics = None;
        if query.mode == SearchMode::HybridRerank {
            rerank_database_results(&mut raw_results, query, &WeightedScoreReranker::default());
            let selection = select_diverse_results(raw_results, query);
            raw_results = selection.results;
            diversity_diagnostics = Some(selection.diagnostics);
        }
        let expanded_results = self.expand_high_level_anchor_context(raw_results, view, query);
        let expanded_results = self.expand_project_related_context(expanded_results, view, query);
        let expanded_results =
            self.expand_search_parent_context(expanded_results, view, query.limit);
        let view_traces = expanded_results
            .iter()
            .filter_map(|result| traces_by_cell.get(&result.cell_id).cloned())
            .collect::<Vec<_>>();
        let ann_report = snapshot_ann_report(
            self,
            query,
            vector_candidates,
            expanded_results.len(),
            policy.unwrap_or_default(),
        );
        Ok(DatabaseSearchOutcome {
            results: expanded_results,
            ann_report,
            view_traces,
            diversity_diagnostics,
        })
    }
}
