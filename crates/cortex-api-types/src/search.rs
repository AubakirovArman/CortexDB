use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SearchResultResponse {
    pub cell_id: u64,
    pub score: u64,
    pub lexical_score: u64,
    pub vector_score: u64,
    pub payload: String,
}

pub type SearchResult = SearchResultResponse;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct AnnSearchReportResponse {
    pub path: String,
    pub fallback_reason: Option<String>,
    #[serde(default)]
    pub fallback_performed: bool,
    pub requested_limit: usize,
    pub allowed_candidates: usize,
    pub graph_nodes: usize,
    pub returned_candidates: usize,
    #[serde(default)]
    pub visited_candidates: usize,
    #[serde(default)]
    pub max_visited_candidates: Option<usize>,
    pub recall_q16: Option<u16>,
    pub min_recall_q16: Option<u16>,
    #[serde(default)]
    pub hnsw_max_neighbors: usize,
    #[serde(default)]
    pub hnsw_ef_search: usize,
    #[serde(default)]
    pub hnsw_ef_construction: usize,
    #[serde(default)]
    pub hnsw_layer_count: usize,
    #[serde(default)]
    pub upper_graph_edges: usize,
    #[serde(default)]
    pub require_slo: bool,
    #[serde(default = "default_production_safe")]
    pub production_safe: bool,
    #[serde(default)]
    pub slo_violations: Vec<String>,
}

pub type AnnSearchReport = AnnSearchReportResponse;

fn default_production_safe() -> bool {
    true
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct AnnNoFallbackDecisionResponse {
    pub allowed: bool,
    #[serde(default)]
    pub reasons: Vec<String>,
}

pub type AnnNoFallbackDecision = AnnNoFallbackDecisionResponse;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct HnswNoFallbackProfileResponse {
    pub configured: bool,
    pub rollout_enabled: Option<bool>,
    pub min_recall_q16: Option<u16>,
    pub require_upper_layers: Option<bool>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SearchRoutingDecisionResponse {
    pub requested_mode: String,
    pub selected_strategy: String,
    pub reason: String,
    pub text_available: bool,
    pub vector_available: bool,
}

pub type SearchRoutingDecision = SearchRoutingDecisionResponse;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SearchResponse {
    pub search_mode: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub routing: Option<SearchRoutingDecisionResponse>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rerank: Option<String>,
    pub ann_report: Option<AnnSearchReportResponse>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub no_fallback_decision: Option<AnnNoFallbackDecisionResponse>,
    pub results: Vec<SearchResultResponse>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SearchExplainTermContributionResponse {
    pub term: String,
    pub term_frequency: u32,
    pub score: u64,
}

pub type SearchExplainTermContribution = SearchExplainTermContributionResponse;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SearchExplainItemResponse {
    pub cell_id: u64,
    #[serde(default)]
    pub rank: usize,
    pub score: u64,
    pub lexical_score: u64,
    pub vector_score: u64,
    #[serde(default)]
    pub lexical_contribution_q16: u16,
    #[serde(default)]
    pub vector_contribution_q16: u16,
    #[serde(default)]
    pub fusion_rank_score: u64,
    #[serde(default)]
    pub matched_terms: Vec<String>,
    #[serde(default)]
    pub matched_fields: Vec<String>,
    #[serde(default)]
    pub term_contributions: Vec<SearchExplainTermContributionResponse>,
    #[serde(default)]
    pub contribution_summary: String,
    pub payload_preview: String,
}

pub type SearchExplainItem = SearchExplainItemResponse;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SearchExplainResponse {
    pub query_terms: Vec<String>,
    pub search_mode: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub routing: Option<SearchRoutingDecisionResponse>,
    pub results: Vec<SearchExplainItemResponse>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
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

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct AnswerGroundingReportResponse {
    pub answer_supported: bool,
    pub rejected: bool,
    pub support_q16: u16,
    pub supported_span_count: u32,
    pub unsupported_span_count: u32,
    pub spans: Vec<AnswerGroundingSpanResponse>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct AnnEvaluationResponse {
    pub available: bool,
    pub reason: Option<String>,
    pub ann_report: Option<AnnSearchReportResponse>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub no_fallback_decision: Option<AnnNoFallbackDecisionResponse>,
    pub exact_top_k: Vec<u32>,
    pub ann_top_k: Vec<u32>,
    pub overlap_count: usize,
    pub recall_q16: u16,
}
