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
pub mod bm25;
mod conditions;
mod database;
mod decomposition;
mod evaluation;
pub(crate) mod frozen_weights;
mod hnsw;
mod hnsw_no_fallback;
mod hnsw_policy;
mod indexes;
mod intent;
mod lexical;
mod persisted;
mod quality_tests;
mod query_understanding;
mod ranking;
mod rerank;
mod routing;
mod scope_mapping;
mod synonyms;
mod tokenizer;
mod types;
pub(crate) mod vector;
mod vector_index;
pub(crate) mod vector_similarity;

pub use analyzer::{
    mean_reciprocal_rank_q16, Language, TextAnalyzer, TextAnalyzerConfig, TEXT_ANALYZER_VERSION,
};
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
pub use bm25::{
    bm25_idf_q16, bm25_term_score_q16, bm25_term_score_with_idf_q16, Bm25Config, Bm25CorpusStats,
    Bm25TermInput, DEFAULT_BM25_B_Q16, DEFAULT_BM25_K1_Q16,
};
pub use conditions::{
    condition_payload_bonus, extract_query_conditions, NumericConditionOperator,
    QueryConditionExtraction, QueryConditionSlot, QueryNumericCondition,
};
pub use database::{
    DatabaseSearchOutcome, DatabaseSearchResult, SearchDiversityDiagnostics, SearchLimit,
    SearchViewTrace,
};
pub(crate) use database::{LiveSearchStore, SearchContextStore};
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
pub use indexes::SearchIndexes;
pub use intent::{
    classify_enterprise_rag_question, classify_enterprise_rag_question_type,
    EnterpriseRagIntentClassification, EnterpriseRagQuestionType,
};
pub use lexical::Bm25Index;
pub use query_understanding::{
    analyze_search_query, analyze_search_query_with_analyzer, QueryAnchor, QueryAnchorKind,
    SearchQueryUnderstanding,
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
pub(crate) use synonyms::CorpusSynonymStore;
pub use synonyms::{
    build_corpus_synonym_dictionary, read_acsyn_dictionary, write_acsyn_dictionary,
    CorpusSynonymCandidate, CorpusSynonymDictionary, CorpusSynonymDictionaryBuilder,
    CorpusSynonymEntry, CorpusSynonymOptions,
};
pub use tokenizer::tokenize;
pub use types::{ScoredCandidate, SearchMode, SearchQuery, SearchResult};
pub use vector::parse_vector_literal;
pub use vector_index::VectorIndex;

use ranking::ranked;
