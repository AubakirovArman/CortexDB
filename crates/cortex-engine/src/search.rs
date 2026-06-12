use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet};

use crate::query::metadata::lexical_field_weight;

mod access;
mod analyzer;
mod ann;
mod ann_corpus;
mod ann_drift;
mod ann_external;
mod ann_fixture;
mod ann_metric_matrix;
mod ann_recall_tests;
mod ann_report;
mod conditions;
mod database;
mod decomposition;
mod evaluation;
mod hnsw;
mod hnsw_no_fallback;
mod hnsw_policy;
mod intent;
mod persisted;
mod quality_tests;
mod query_understanding;
mod rerank;
mod routing;
mod scope_mapping;
mod synonyms;
mod tokenizer;
pub(crate) mod vector;

pub use analyzer::{mean_reciprocal_rank_q16, Language, TextAnalyzer};
pub use ann::{
    AnnEvaluationReport, AnnFallbackReason, AnnMetrics, AnnSearchPath, AnnSearchPolicy,
    AnnSearchReport, AnnSloViolation, MIN_ANN_RECALL_Q16,
};
pub use ann_corpus::{
    evaluate_ann_corpus, metric_name, parse_ann_metric, AnnCorpusGroundTruth, AnnCorpusOptions,
    AnnCorpusQuery, AnnCorpusQueryReport, AnnCorpusReport, AnnCorpusVector,
};
pub use ann_drift::{
    compare_ann_drift_baseline, evaluate_ann_drift_baseline, AnnDriftBaseline, AnnDriftReport,
};
pub use ann_external::{
    evaluate_ann_external_fixture, AnnExternalFixtureBaseline, AnnExternalFixtureReport,
    AnnJsonlEntry,
};
pub use ann_fixture::{
    compare_ann_fixture_baseline, evaluate_ann_fixture_baseline, AnnRecallLatencyBaseline,
    AnnRecallLatencyGateReport,
};
pub use ann_metric_matrix::{
    evaluate_ann_metric_matrix, AnnMetricBaseline, AnnMetricMatrixBaseline, AnnMetricMatrixReport,
    AnnMetricReport,
};
pub use ann_report::{
    synthetic_ann_recall_latency_report, AnnRecallLatencyReport, SYNTHETIC_ANN_CORPUS_V1,
};
pub use conditions::{
    condition_payload_bonus, extract_query_conditions, NumericConditionOperator,
    QueryConditionExtraction, QueryConditionSlot, QueryNumericCondition,
};
pub use database::{
    DatabaseSearchOutcome, DatabaseSearchResult, SearchDiversityDiagnostics, SearchLimit,
    SearchViewTrace,
};
pub use decomposition::{
    covered_requirement_ids, decompose_enterprise_rag_question, split_subquestions,
    QuestionDecomposition, QuestionRequirement, QuestionRequirementKind,
};
pub use hnsw::{integrity::HnswIntegrityReport, DistanceMetric, HnswIndex, VectorCollectionConfig};
pub use hnsw_no_fallback::{
    evaluate_hnsw_no_fallback_rollout, HnswNoFallbackBlockReason, HnswNoFallbackDecision,
    HnswNoFallbackRolloutPolicy,
};
pub use hnsw_policy::{
    HnswBuildConfig, HnswBuildProfile, HnswMaintenancePolicy, HnswMaintenanceReport,
    HnswRebuildPolicy,
};
pub use intent::{
    classify_enterprise_rag_question, classify_enterprise_rag_question_type,
    EnterpriseRagIntentClassification, EnterpriseRagQuestionType,
};
pub use query_understanding::{
    analyze_search_query, QueryAnchor, QueryAnchorKind, SearchQueryUnderstanding,
};
pub use rerank::{
    calibrated_hybrid_rrf_weights, rerank_calibration_profile, HybridRrfWeights,
    RerankCalibrationProfile, SearchRerankInput, SearchReranker, WeightedScoreReranker,
};
pub use routing::{
    classify_search_query_intent, route_policy_for_query, route_search_query,
    route_search_query_for_text, routed_candidate_limit, routed_result_limit, routed_token_budget,
    SearchQueryIntent, SearchRouteDecision, SearchRouteInput, SearchRoutePolicy,
    SearchRouteStrategy,
};
pub use scope_mapping::{
    map_query_to_scope, scope_mapping_metadata_bonus, scope_mapping_payload_bonus,
    QueryScopeDirective, QueryScopeField, QueryScopeMapping,
};
pub use synonyms::{
    build_corpus_synonym_dictionary, read_acsyn_dictionary, write_acsyn_dictionary,
    CorpusSynonymCandidate, CorpusSynonymDictionary, CorpusSynonymDictionaryBuilder,
    CorpusSynonymEntry, CorpusSynonymOptions,
};
pub use tokenizer::tokenize;
pub use vector::parse_vector_literal;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScoredCandidate {
    pub cell_id: u32,
    pub score: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SearchMode {
    Keyword,
    Vector,
    VectorExact,
    Hybrid,
    HybridRerank,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SearchQuery<'a> {
    pub text: &'a str,
    pub vector: Option<&'a [i16]>,
    pub limit: usize,
    pub mode: SearchMode,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SearchResult {
    pub cell_id: u32,
    pub score: u64,
    pub lexical_score: u64,
    pub vector_score: u64,
}

#[derive(Clone, Debug, Default)]
pub struct Bm25Index {
    docs: BTreeMap<u32, BTreeMap<String, u32>>,
    doc_lengths: BTreeMap<u32, u32>,
    postings: BTreeMap<String, BTreeSet<u32>>,
    field_docs: BTreeMap<u32, BTreeMap<String, BTreeMap<String, u32>>>,
    field_doc_lengths: BTreeMap<String, BTreeMap<u32, u32>>,
}

#[derive(Clone, Debug, Default)]
pub struct VectorIndex {
    vectors: BTreeMap<u32, Vec<i16>>,
    metric: hnsw::DistanceMetric,
}

#[derive(Clone, Debug, Default)]
pub struct SearchIndexes {
    pub lexical: Bm25Index,
    pub vector: VectorIndex,
}

impl Bm25Index {
    pub fn add_document(&mut self, cell_id: u32, text: &str) {
        self.add_document_fields(cell_id, &[(text, 1)]);
    }

    pub fn add_document_fields(&mut self, cell_id: u32, fields: &[(&str, u32)]) {
        let mut terms = BTreeMap::<String, u32>::new();
        for (text, weight) in fields {
            let weight = (*weight).max(1);
            for term in tokenize(text) {
                *terms.entry(term).or_default() += weight;
            }
        }
        self.replace_document_terms(cell_id, terms);
        self.clear_field_terms(cell_id);
    }

    pub fn add_weighted_terms(&mut self, cell_id: u32, terms: BTreeMap<String, u32>) {
        self.replace_document_terms(cell_id, terms);
        self.clear_field_terms(cell_id);
    }

    pub fn add_field_terms(
        &mut self,
        cell_id: u32,
        fields: BTreeMap<String, BTreeMap<String, u32>>,
    ) {
        self.replace_document_terms(cell_id, weighted_terms_from_fields(&fields));
        self.replace_field_terms(cell_id, fields);
    }

    fn replace_document_terms(&mut self, cell_id: u32, terms: BTreeMap<String, u32>) {
        if let Some(old_terms) = self.docs.get(&cell_id) {
            for term in old_terms.keys() {
                if let Some(posting) = self.postings.get_mut(term) {
                    posting.remove(&cell_id);
                }
            }
            self.postings.retain(|_, posting| !posting.is_empty());
        }
        for term in terms.keys() {
            self.postings
                .entry(term.clone())
                .or_default()
                .insert(cell_id);
        }
        self.doc_lengths
            .insert(cell_id, terms.values().copied().sum::<u32>().max(1));
        self.docs.insert(cell_id, terms);
    }

    fn replace_field_terms(
        &mut self,
        cell_id: u32,
        fields: BTreeMap<String, BTreeMap<String, u32>>,
    ) {
        self.clear_field_terms(cell_id);
        for (field, terms) in &fields {
            self.field_doc_lengths
                .entry(field.clone())
                .or_default()
                .insert(cell_id, terms.values().copied().sum::<u32>().max(1));
        }
        self.field_docs.insert(cell_id, fields);
    }

    fn clear_field_terms(&mut self, cell_id: u32) {
        let Some(old_fields) = self.field_docs.remove(&cell_id) else {
            return;
        };
        for field in old_fields.keys() {
            if let Some(lengths) = self.field_doc_lengths.get_mut(field) {
                lengths.remove(&cell_id);
            }
        }
        self.field_doc_lengths
            .retain(|_, lengths| !lengths.is_empty());
    }

    pub fn search(&self, query: &str, limit: usize) -> Vec<ScoredCandidate> {
        let analyzed = query_understanding::analyze_search_query(query);
        let query_terms = analyzed.weighted_terms;
        let doc_count = self.docs.len() as u64;
        let avg_len_q10 = self.average_len_q10();
        let mut scores = BTreeMap::<u32, u64>::new();
        for (term, query_weight) in query_terms {
            let Some(posting) = self.postings.get(&term) else {
                continue;
            };
            let idf_q10 = ((doc_count + 1) * 1024) / (posting.len() as u64 + 1);
            for cell_id in posting {
                let doc = &self.docs[cell_id];
                let field_score =
                    self.field_score_q10(*cell_id, &term, idf_q10, u64::from(query_weight));
                let score = if field_score > 0 {
                    field_score
                } else {
                    let tf = u64::from(*doc.get(&term).unwrap_or(&0));
                    let len_q10 = u64::from(*self.doc_lengths.get(cell_id).unwrap_or(&1)) * 1024;
                    let norm_q10 = 256 + (768 * len_q10 / avg_len_q10.max(1));
                    let denom_q10 = (tf * 1024) + norm_q10;
                    let tf_norm_q10 = (tf * 2048 * 1024) / denom_q10.max(1);
                    idf_q10 * tf_norm_q10 * u64::from(query_weight)
                };
                *scores.entry(*cell_id).or_default() += score;
            }
        }
        ranked(scores, limit)
    }

    fn field_score_q10(&self, cell_id: u32, term: &str, idf_q10: u64, query_weight: u64) -> u64 {
        let Some(fields) = self.field_docs.get(&cell_id) else {
            return 0;
        };
        fields
            .iter()
            .filter_map(|(field, terms)| terms.get(term).map(|tf| (field, *tf)))
            .map(|(field, tf)| {
                let tf = u64::from(tf);
                let len_q10 = self
                    .field_doc_lengths
                    .get(field)
                    .and_then(|lengths| lengths.get(&cell_id))
                    .copied()
                    .map(u64::from)
                    .unwrap_or(1)
                    * 1024;
                let avg_len_q10 = self.average_field_len_q10(field);
                let norm_q10 = 256 + (768 * len_q10 / avg_len_q10.max(1));
                let denom_q10 = (tf * 1024) + norm_q10;
                let tf_norm_q10 = (tf * 2048 * 1024) / denom_q10.max(1);
                idf_q10
                    .saturating_mul(tf_norm_q10)
                    .saturating_mul(query_weight)
                    .saturating_mul(u64::from(lexical_field_weight(field)))
            })
            .sum()
    }

    fn average_len_q10(&self) -> u64 {
        let total = self
            .doc_lengths
            .values()
            .copied()
            .map(u64::from)
            .sum::<u64>();
        if self.doc_lengths.is_empty() {
            1024
        } else {
            total * 1024 / self.doc_lengths.len() as u64
        }
    }

    fn average_field_len_q10(&self, field: &str) -> u64 {
        let Some(lengths) = self.field_doc_lengths.get(field) else {
            return 1024;
        };
        let total = lengths.values().copied().map(u64::from).sum::<u64>();
        if lengths.is_empty() {
            1024
        } else {
            total * 1024 / lengths.len() as u64
        }
    }
}

impl SearchIndexes {
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
        let mut results = self.hybrid_search_with_weights(
            SearchQuery {
                mode: SearchMode::Hybrid,
                limit: rerank_candidate_limit(query),
                ..query
            },
            HybridRrfWeights::balanced(),
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
        let lexical = self.lexical.search(query.text, query.limit.max(32));
        let vector = self.vector.search_dot(vector, query.limit.max(32));
        let mut results = BTreeMap::<u32, SearchResult>::new();
        apply_rrf(&mut results, lexical, true, weights.lexical_q16);
        apply_rrf(&mut results, vector, false, weights.vector_q16);
        let mut values = results.into_values().collect::<Vec<_>>();
        values.sort_by_key(|result| (Reverse(result.score), result.cell_id));
        values.truncate(query.limit);
        values
    }
}

fn weighted_terms_from_fields(
    fields: &BTreeMap<String, BTreeMap<String, u32>>,
) -> BTreeMap<String, u32> {
    let mut terms = BTreeMap::new();
    for (field, field_terms) in fields {
        let weight = lexical_field_weight(field);
        for (term, frequency) in field_terms {
            *terms.entry(term.clone()).or_default() += frequency.saturating_mul(weight);
        }
    }
    terms
}

fn rerank_base_mode(mode: SearchMode) -> SearchMode {
    match mode {
        SearchMode::HybridRerank => SearchMode::Hybrid,
        other => other,
    }
}

fn rerank_candidate_limit(query: SearchQuery<'_>) -> usize {
    routed_candidate_limit(query.text, query.limit).max(query.limit.max(32))
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

impl VectorIndex {
    pub fn add_vector(&mut self, cell_id: u32, vector: Vec<i16>) {
        self.vectors.insert(cell_id, vector);
    }

    pub fn search_dot(&self, query: &[i16], limit: usize) -> Vec<ScoredCandidate> {
        let scores = self
            .vectors
            .iter()
            .filter_map(|(cell_id, vector)| {
                self.metric
                    .distance(query, vector)
                    .map(|score| (*cell_id, score))
            })
            .collect();
        ranked(scores, limit)
    }
}

pub(super) fn ranked(scores: BTreeMap<u32, u64>, limit: usize) -> Vec<ScoredCandidate> {
    if limit == 0 {
        return Vec::new();
    }
    let mut values: Vec<_> = scores
        .into_iter()
        .map(|(cell_id, score)| ScoredCandidate { cell_id, score })
        .collect();
    if values.len() > limit {
        values.select_nth_unstable_by_key(limit, |candidate| {
            (Reverse(candidate.score), candidate.cell_id)
        });
        values.truncate(limit);
    }
    values.sort_by_key(|candidate| (Reverse(candidate.score), candidate.cell_id));
    values
}

fn apply_rrf(
    results: &mut BTreeMap<u32, SearchResult>,
    ranked: Vec<ScoredCandidate>,
    lexical: bool,
    weight_q16: u32,
) {
    for (rank, candidate) in ranked.into_iter().enumerate() {
        let rrf =
            (1_000_000 / (60 + rank as u64 + 1)).saturating_mul(u64::from(weight_q16)) / 65_535;
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

#[cfg(test)]
mod tests {
    use super::{ranked, ScoredCandidate};
    use std::collections::BTreeMap;

    #[test]
    fn ranked_returns_empty_for_zero_limit() {
        let results = ranked(BTreeMap::from([(1, 10), (2, 20)]), 0);
        assert!(results.is_empty());
    }

    #[test]
    fn ranked_keeps_deterministic_top_n_without_full_output() {
        let results = ranked(
            BTreeMap::from([(1, 10), (2, 40), (3, 40), (4, 20), (5, 30)]),
            3,
        );

        assert_eq!(
            results,
            vec![
                ScoredCandidate {
                    cell_id: 2,
                    score: 40,
                },
                ScoredCandidate {
                    cell_id: 3,
                    score: 40,
                },
                ScoredCandidate {
                    cell_id: 5,
                    score: 30,
                },
            ]
        );
    }
}
