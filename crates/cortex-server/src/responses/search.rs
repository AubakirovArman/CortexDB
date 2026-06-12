use serde::Serialize;

#[derive(Serialize, Debug, Clone)]
pub struct SearchResultResponse {
    pub cell_id: u64,
    pub score: u64,
    pub lexical_score: u64,
    pub vector_score: u64,
    pub payload: String,
}

#[derive(Serialize, Debug, Clone)]
pub struct AnnSearchReportResponse {
    pub path: String,
    pub fallback_reason: Option<String>,
    pub fallback_performed: bool,
    pub requested_limit: usize,
    pub allowed_candidates: usize,
    pub graph_nodes: usize,
    pub returned_candidates: usize,
    pub visited_candidates: usize,
    pub max_visited_candidates: Option<usize>,
    pub recall_q16: Option<u16>,
    pub min_recall_q16: Option<u16>,
    pub hnsw_max_neighbors: usize,
    pub hnsw_ef_search: usize,
    pub hnsw_ef_construction: usize,
    pub hnsw_layer_count: usize,
    pub upper_graph_edges: usize,
    pub require_slo: bool,
    pub production_safe: bool,
    pub slo_violations: Vec<String>,
}

#[derive(Serialize, Debug, Clone)]
pub struct AnnNoFallbackDecisionResponse {
    pub allowed: bool,
    pub reasons: Vec<String>,
}

#[derive(Serialize, Debug, Clone)]
pub struct HnswNoFallbackProfileResponse {
    pub configured: bool,
    pub rollout_enabled: Option<bool>,
    pub min_recall_q16: Option<u16>,
    pub require_upper_layers: Option<bool>,
}

#[derive(Serialize, Debug, Clone)]
pub struct SearchRoutingDecisionResponse {
    pub requested_mode: String,
    pub selected_strategy: String,
    pub reason: String,
    pub text_available: bool,
    pub vector_available: bool,
}

#[derive(Serialize, Debug, Clone)]
pub struct SearchResponse {
    pub search_mode: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub routing: Option<SearchRoutingDecisionResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rerank: Option<String>,
    pub ann_report: Option<AnnSearchReportResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub no_fallback_decision: Option<AnnNoFallbackDecisionResponse>,
    pub results: Vec<SearchResultResponse>,
}

#[derive(Serialize, Debug, Clone)]
pub struct SearchExplainTermContributionResponse {
    pub term: String,
    pub term_frequency: u32,
    pub score: u64,
}

#[derive(Serialize, Debug, Clone)]
pub struct SearchExplainItemResponse {
    pub cell_id: u64,
    pub rank: usize,
    pub score: u64,
    pub lexical_score: u64,
    pub vector_score: u64,
    pub lexical_contribution_q16: u16,
    pub vector_contribution_q16: u16,
    pub fusion_rank_score: u64,
    pub matched_terms: Vec<String>,
    pub matched_fields: Vec<String>,
    pub term_contributions: Vec<SearchExplainTermContributionResponse>,
    pub contribution_summary: String,
    pub payload_preview: String,
}

#[derive(Serialize, Debug, Clone)]
pub struct SearchExplainResponse {
    pub query_terms: Vec<String>,
    pub search_mode: String,
    pub routing: SearchRoutingDecisionResponse,
    pub results: Vec<SearchExplainItemResponse>,
}

#[derive(Serialize, Debug, Clone)]
pub struct AnnEvaluationResponse {
    pub available: bool,
    pub reason: Option<String>,
    pub ann_report: Option<AnnSearchReportResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub no_fallback_decision: Option<AnnNoFallbackDecisionResponse>,
    pub exact_top_k: Vec<u32>,
    pub ann_top_k: Vec<u32>,
    pub overlap_count: usize,
    pub recall_q16: u16,
}

#[derive(Serialize, Debug, Clone)]
pub struct LlmInferenceAuditResponse {
    pub context_pack_only: bool,
    pub prompt_body_logged: bool,
    pub secrets_logged: bool,
}

#[derive(Serialize, Debug, Clone)]
pub struct AnswerGroundingSpanResponse {
    pub text: String,
    pub start_byte: usize,
    pub end_byte: usize,
    pub support_q16: u16,
    pub supported: bool,
    pub covered_terms: Vec<String>,
    pub missing_terms: Vec<String>,
    pub supported_by_cell_ids: Vec<u64>,
    pub citations: Vec<String>,
}

#[derive(Serialize, Debug, Clone)]
pub struct AnswerGroundingReportResponse {
    pub answer_supported: bool,
    pub rejected: bool,
    pub support_q16: u16,
    pub supported_span_count: u32,
    pub unsupported_span_count: u32,
    pub spans: Vec<AnswerGroundingSpanResponse>,
}

#[derive(Serialize, Debug, Clone)]
pub struct LlmInferenceResponse {
    pub schema_version: &'static str,
    pub provider: String,
    pub model: String,
    pub output: String,
    pub used_context_cell_ids: Vec<u64>,
    pub citations: Vec<String>,
    pub grounding: AnswerGroundingReportResponse,
    pub audit: LlmInferenceAuditResponse,
}
