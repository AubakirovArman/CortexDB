use std::collections::BTreeMap;

use cortex_aql::AgentView;

use crate::database::Database;
use crate::error::{EngineError, EngineResult};
use crate::options::PayloadResidency;
use crate::plan::PolicyRewrite;

use super::super::ann::AnnSearchPolicy;
use super::super::{SearchIndexes, SearchMode, SearchQuery, WeightedScoreReranker};
use super::ann_reports::snapshot_ann_report;
use super::diversity::select_diverse_results;
use super::ranking::{database_index_query, rerank_database_results};
use super::trace::trace_search;
use super::{DatabaseSearchOutcome, SearchViewTrace};

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
        let analyzer = self.text_analyzer();
        let mut indexes = SearchIndexes::with_text_analyzer_config(self.text_analyzer_config);
        let mut cells = BTreeMap::new();
        let mut traces = BTreeMap::<u32, SearchViewTrace>::new();
        let mut vector_candidates = 0usize;
        for (index, record) in self
            .snapshot_search_records(view, query)?
            .into_iter()
            .enumerate()
        {
            let candidate_id =
                u32::try_from(index + 1).map_err(|_| EngineError::CandidateIdOverflow)?;
            indexes.add_field_terms(
                candidate_id,
                record.metadata.lexical_field_terms_with_analyzer(&analyzer),
            );
            if let Some(best) = record.best_vector.as_ref() {
                if let Some(trace) = record.trace(candidate_id) {
                    traces.insert(candidate_id, trace);
                }
                indexes.add_vector(candidate_id, best.vector.clone());
                vector_candidates += 1;
            }
            cells.insert(candidate_id, record);
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
                let candidate = cells.remove(&candidate_id)?;
                Some((
                    candidate_id,
                    candidate.into_result(result.score, result.lexical_score, result.vector_score),
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

    fn snapshot_search_records(
        &self,
        view: &AgentView,
        query: SearchQuery<'_>,
    ) -> EngineResult<Vec<super::live_store::LiveSearchCandidate>> {
        if self.payload_residency != PayloadResidency::Lazy {
            return Ok(self.live_search_store.visible_records(view, query.vector));
        }

        let txn = self.read_txn();
        let mut records = Vec::new();
        for version in self.memtable.visible_iter(txn) {
            let payload = self.payload_for_version(version)?;
            let record = super::LiveSearchStore::candidate_from_payload(
                version.cell_id,
                payload,
                &version.descriptor,
                query.vector,
            );
            if PolicyRewrite::allows_scope(view, crate::query::scope_id(&record.metadata.scope)) {
                records.push(record);
            }
        }
        Ok(records)
    }
}
