use std::cmp::Reverse;
use std::collections::BTreeMap;

use super::{
    frozen_weights, rerank, routed_candidate_limit, routed_result_limit, Bm25Index,
    HybridRrfWeights, ScoredCandidate, SearchMode, SearchQuery, SearchRerankInput, SearchReranker,
    SearchResult, TextAnalyzerConfig, VectorIndex, WeightedScoreReranker,
};

#[derive(Clone, Debug, Default)]
pub struct SearchIndexes {
    pub lexical: Bm25Index,
    pub vector: VectorIndex,
}

impl SearchIndexes {
    pub fn with_text_analyzer_config(analyzer_config: TextAnalyzerConfig) -> Self {
        Self {
            lexical: Bm25Index::with_analyzer_config(analyzer_config),
            vector: VectorIndex::default(),
        }
    }

    pub fn add_document(&mut self, cell_id: u32, text: &str) {
        self.lexical.add_document(cell_id, text);
    }

    pub fn add_document_fields(&mut self, cell_id: u32, fields: &[(&str, u32)]) {
        self.lexical.add_document_fields(cell_id, fields);
    }

    pub fn add_weighted_terms(&mut self, cell_id: u32, terms: BTreeMap<String, u32>) {
        self.lexical.add_weighted_terms(cell_id, terms);
    }

    pub fn add_field_terms(
        &mut self,
        cell_id: u32,
        fields: BTreeMap<String, BTreeMap<String, u32>>,
    ) {
        self.lexical.add_field_terms(cell_id, fields);
    }

    pub fn add_vector(&mut self, cell_id: u32, vector: Vec<i16>) {
        self.vector.add_vector(cell_id, vector);
    }

    pub fn search(&self, query: SearchQuery<'_>) -> Vec<SearchResult> {
        match query.mode {
            SearchMode::Keyword => self
                .lexical
                .search(query.text, query.limit)
                .into_iter()
                .map(SearchResult::from_lexical)
                .collect(),
            SearchMode::Vector | SearchMode::VectorExact => query
                .vector
                .map(|vector| self.vector.search_dot(vector, query.limit))
                .unwrap_or_default()
                .into_iter()
                .map(SearchResult::from_vector)
                .collect(),
            SearchMode::Hybrid => self.hybrid_search(query),
            SearchMode::HybridRerank => self.hybrid_rerank_search(query),
        }
    }

    pub fn search_with_hybrid_rrf_weights(
        &self,
        query: SearchQuery<'_>,
        weights: HybridRrfWeights,
    ) -> Vec<SearchResult> {
        match query.mode {
            SearchMode::Hybrid => self.hybrid_search_with_weights(query, weights),
            SearchMode::HybridRerank => self.hybrid_rerank_search_with_weights(query, weights),
            _ => self.search(query),
        }
    }

    pub fn search_with_reranker(
        &self,
        query: SearchQuery<'_>,
        reranker: &dyn SearchReranker,
    ) -> Vec<SearchResult> {
        let mut results = self.search(SearchQuery {
            mode: rerank_base_mode(query.mode),
            limit: rerank_candidate_limit(query),
            ..query
        });
        rerank_results(&mut results, query, reranker);
        results.truncate(rerank_result_limit(query));
        results
    }

    fn hybrid_rerank_search(&self, query: SearchQuery<'_>) -> Vec<SearchResult> {
        self.hybrid_rerank_search_with_weights(query, HybridRrfWeights::balanced())
    }

    fn hybrid_rerank_search_with_weights(
        &self,
        query: SearchQuery<'_>,
        weights: HybridRrfWeights,
    ) -> Vec<SearchResult> {
        let mut results = self.hybrid_search_with_weights(
            SearchQuery {
                mode: SearchMode::Hybrid,
                limit: rerank_candidate_limit(query),
                ..query
            },
            weights,
        );
        rerank_results(&mut results, query, &WeightedScoreReranker::fixed_default());
        results.truncate(rerank_result_limit(query));
        results
    }

    fn hybrid_search(&self, query: SearchQuery<'_>) -> Vec<SearchResult> {
        self.hybrid_search_with_weights(query, HybridRrfWeights::balanced())
    }

    fn hybrid_search_with_weights(
        &self,
        query: SearchQuery<'_>,
        weights: HybridRrfWeights,
    ) -> Vec<SearchResult> {
        let Some(vector) = query.vector else {
            return self.search(SearchQuery {
                mode: SearchMode::Keyword,
                ..query
            });
        };
        let lexical = self.lexical.search(
            query.text,
            query.limit.max(frozen_weights::RERANK_MIN_CANDIDATE_LIMIT),
        );
        let vector = self.vector.search_dot(
            vector,
            query.limit.max(frozen_weights::RERANK_MIN_CANDIDATE_LIMIT),
        );
        let mut results = BTreeMap::<u32, SearchResult>::new();
        apply_rrf(&mut results, lexical, true, weights.lexical_q16);
        apply_rrf(&mut results, vector, false, weights.vector_q16);
        let mut values = results.into_values().collect::<Vec<_>>();
        values.sort_by_key(|result| (Reverse(result.score), result.cell_id));
        values.truncate(query.limit);
        values
    }
}

fn rerank_base_mode(mode: SearchMode) -> SearchMode {
    match mode {
        SearchMode::HybridRerank => SearchMode::Hybrid,
        other => other,
    }
}

fn rerank_candidate_limit(query: SearchQuery<'_>) -> usize {
    routed_candidate_limit(query.text, query.limit)
        .max(query.limit.max(frozen_weights::RERANK_MIN_CANDIDATE_LIMIT))
}

fn rerank_result_limit(query: SearchQuery<'_>) -> usize {
    routed_result_limit(query.text, query.limit)
}

fn rerank_results(
    results: &mut [SearchResult],
    query: SearchQuery<'_>,
    reranker: &dyn SearchReranker,
) {
    for result in results.iter_mut() {
        result.score = reranker.rerank_score(SearchRerankInput {
            query_text: query.text,
            query_vector: query.vector,
            candidate_id: u64::from(result.cell_id),
            lexical_score: result.lexical_score,
            vector_score: result.vector_score,
            base_score: result.score,
            metadata: None,
            payload: None,
        });
    }
    rerank::sort_reranked(results, |result| result.cell_id, |result| result.score);
}

impl SearchResult {
    fn from_lexical(candidate: ScoredCandidate) -> Self {
        Self {
            cell_id: candidate.cell_id,
            score: candidate.score,
            lexical_score: candidate.score,
            vector_score: 0,
        }
    }

    fn from_vector(candidate: ScoredCandidate) -> Self {
        Self {
            cell_id: candidate.cell_id,
            score: candidate.score,
            lexical_score: 0,
            vector_score: candidate.score,
        }
    }
}

fn apply_rrf(
    results: &mut BTreeMap<u32, SearchResult>,
    ranked: Vec<ScoredCandidate>,
    lexical: bool,
    weight_q16: u32,
) {
    for (rank, candidate) in ranked.into_iter().enumerate() {
        let rrf = (frozen_weights::RRF_SCORE_SCALE
            / (frozen_weights::RRF_RANK_CONSTANT + rank as u64 + 1))
            .saturating_mul(u64::from(weight_q16))
            / frozen_weights::Q16_ONE_U64;
        let result = results.entry(candidate.cell_id).or_insert(SearchResult {
            cell_id: candidate.cell_id,
            score: 0,
            lexical_score: 0,
            vector_score: 0,
        });
        result.score += rrf;
        if lexical {
            result.lexical_score = candidate.score;
        } else {
            result.vector_score = candidate.score;
        }
    }
}
