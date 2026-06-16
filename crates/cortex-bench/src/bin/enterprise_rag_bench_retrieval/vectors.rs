use std::collections::BTreeMap;

use cortex_aql::AgentView;
use cortex_core::CellId;
use cortex_engine::search::{
    SearchIndexes, SearchMode, SearchQuery, SearchRerankInput, SearchReranker, SearchResult,
};
use cortex_engine::{scope_id, CellMetadata, Database};
use serde_json::json;

use super::logger::{logger_progress_due, RunLogger};

mod lookup;
mod parse;

pub(crate) use lookup::DocumentVectorLookup;
#[cfg(test)]
pub(crate) use parse::load_id_vectors;
pub(crate) use parse::{
    body_vector_from_payload, load_document_vectors, load_query_vectors, parse_query_vector,
    payload_has_vector, vector_dot_score,
};

pub(super) struct BenchmarkSearchIndex {
    indexes: SearchIndexes,
    vectors: BTreeMap<u32, Vec<i16>>,
    candidate_to_cell: BTreeMap<u32, CellId>,
}

impl BenchmarkSearchIndex {
    pub(super) fn load(
        db: &Database,
        uuid_index: &BTreeMap<String, String>,
        view: &AgentView,
        logger: &RunLogger,
    ) -> Result<Self, String> {
        let mut indexes = SearchIndexes::default();
        let mut vectors = BTreeMap::new();
        let mut candidate_to_cell = BTreeMap::new();
        let total = uuid_index.len();
        for (index, doc_id) in uuid_index.keys().enumerate() {
            let candidate =
                u32::try_from(index + 1).map_err(|_| "candidate id overflow".to_owned())?;
            let cell_id = CellId(u64::from(candidate));
            let Some((payload, descriptor)) = db.get_latest_cell_with_descriptor(cell_id) else {
                continue;
            };
            let metadata = CellMetadata::from_payload_with_descriptor(&payload, &descriptor);
            if !view.can_read_scope(scope_id(&metadata.scope)) {
                continue;
            }
            indexes.add_field_terms(candidate, metadata.lexical_field_terms());
            if let Some(vector) = body_vector_from_payload(&payload) {
                vectors.insert(candidate, vector);
            }
            candidate_to_cell.insert(candidate, cell_id);
            if logger_progress_due(index + 1, total, 50_000) {
                logger.log(&format!(
                    "built reusable search index {}/{} last_doc_id={}",
                    index + 1,
                    total,
                    doc_id
                ));
                logger.status(
                    "build_reusable_search_index",
                    "running",
                    "build reusable in-memory search index",
                    Some(index + 1),
                    Some(total),
                    &[("last_doc_id", json!(doc_id))],
                );
            }
        }
        Ok(Self {
            indexes,
            vectors,
            candidate_to_cell,
        })
    }

    pub(super) fn search_payloads(
        &self,
        db: &Database,
        query: SearchQuery<'_>,
        top_k: usize,
        reranker: Option<&dyn SearchReranker>,
    ) -> Vec<Vec<u8>> {
        if query.vector.is_some()
            && matches!(query.mode, SearchMode::Hybrid | SearchMode::HybridRerank)
        {
            return self.search_bounded_hybrid_payloads(db, query, top_k, reranker);
        }
        let candidate_limit = if reranker.is_some() {
            top_k.max(64)
        } else {
            top_k.max(1)
        };
        let search_query = SearchQuery {
            limit: candidate_limit,
            ..query
        };
        let mut results = self.indexes.search(search_query);
        if let Some(reranker) = reranker {
            self.rerank_results(db, &mut results, query, reranker);
        }
        results.truncate(top_k);
        results
            .into_iter()
            .filter_map(|result| {
                let cell_id = self.candidate_to_cell.get(&result.cell_id)?;
                db.get_latest_cell(*cell_id)
            })
            .collect()
    }

    fn search_bounded_hybrid_payloads(
        &self,
        db: &Database,
        query: SearchQuery<'_>,
        top_k: usize,
        reranker: Option<&dyn SearchReranker>,
    ) -> Vec<Vec<u8>> {
        let lexical_pool = top_k.max(2_048);
        let lexical_query = SearchQuery {
            mode: SearchMode::Keyword,
            limit: lexical_pool,
            ..query
        };
        let mut results = self.indexes.search(lexical_query);
        if results.is_empty() {
            results = self
                .vectors
                .keys()
                .take(lexical_pool)
                .map(|candidate| SearchResult {
                    cell_id: *candidate,
                    score: 0,
                    lexical_score: 0,
                    vector_score: 0,
                })
                .collect();
        }
        if let Some(query_vector) = query.vector {
            for result in &mut results {
                if let Some(candidate_vector) = self.vectors.get(&result.cell_id) {
                    result.vector_score = vector_dot_score(query_vector, candidate_vector);
                    result.score = result.lexical_score.saturating_add(result.vector_score);
                }
            }
        }
        if let Some(reranker) = reranker {
            self.rerank_results(db, &mut results, query, reranker);
        } else {
            results.sort_by_key(|result| (std::cmp::Reverse(result.score), result.cell_id));
        }
        results.truncate(top_k);
        results
            .into_iter()
            .filter_map(|result| {
                let cell_id = self.candidate_to_cell.get(&result.cell_id)?;
                db.get_latest_cell(*cell_id)
            })
            .collect()
    }

    fn rerank_results(
        &self,
        db: &Database,
        results: &mut [SearchResult],
        query: SearchQuery<'_>,
        reranker: &dyn SearchReranker,
    ) {
        for result in results.iter_mut() {
            let payload = self
                .candidate_to_cell
                .get(&result.cell_id)
                .and_then(|cell_id| db.get_latest_cell(*cell_id));
            result.score = reranker.rerank_score(SearchRerankInput {
                query_text: query.text,
                query_vector: query.vector,
                candidate_id: u64::from(result.cell_id),
                lexical_score: result.lexical_score,
                vector_score: result.vector_score,
                base_score: result.score,
                metadata: None,
                payload: payload.as_deref(),
            });
        }
        results.sort_by_key(|result| (std::cmp::Reverse(result.score), result.cell_id));
    }
}
