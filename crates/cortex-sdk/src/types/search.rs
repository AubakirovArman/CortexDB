use serde::Deserialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VectorAlgorithm {
    Ann,
    Exact,
}

impl VectorAlgorithm {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Ann => "ann",
            Self::Exact => "exact",
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct AnnSearchReport {
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
    pub hnsw_ef_construction: usize,
    #[serde(default)]
    pub require_slo: bool,
    #[serde(default = "default_production_safe")]
    pub production_safe: bool,
    #[serde(default)]
    pub slo_violations: Vec<String>,
}

fn default_production_safe() -> bool {
    true
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct AnnEvaluationResponse {
    pub available: bool,
    pub reason: Option<String>,
    pub ann_report: Option<AnnSearchReport>,
    #[serde(default)]
    pub no_fallback_decision: Option<AnnNoFallbackDecision>,
    pub exact_top_k: Vec<u32>,
    pub ann_top_k: Vec<u32>,
    pub overlap_count: usize,
    pub recall_q16: u16,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct AnnNoFallbackDecision {
    pub allowed: bool,
    #[serde(default)]
    pub reasons: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct HnswNoFallbackProfileResponse {
    pub configured: bool,
    pub rollout_enabled: Option<bool>,
    pub min_recall_q16: Option<u16>,
    pub require_upper_layers: Option<bool>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct SearchResult {
    pub cell_id: u64,
    pub score: u64,
    pub lexical_score: u64,
    pub vector_score: u64,
    pub payload: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct SearchResponse {
    pub search_mode: String,
    #[serde(default)]
    pub routing: Option<SearchRoutingDecision>,
    #[serde(default)]
    pub rerank: Option<String>,
    pub ann_report: Option<AnnSearchReport>,
    #[serde(default)]
    pub no_fallback_decision: Option<AnnNoFallbackDecision>,
    pub results: Vec<SearchResult>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct SearchRoutingDecision {
    pub requested_mode: String,
    pub selected_strategy: String,
    pub reason: String,
    pub text_available: bool,
    pub vector_available: bool,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct SearchExplainTermContribution {
    pub term: String,
    pub term_frequency: u32,
    pub score: u64,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct SearchExplainItem {
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
    pub term_contributions: Vec<SearchExplainTermContribution>,
    #[serde(default)]
    pub contribution_summary: String,
    pub payload_preview: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct SearchExplainResponse {
    pub query_terms: Vec<String>,
    pub search_mode: String,
    #[serde(default)]
    pub routing: Option<SearchRoutingDecision>,
    pub results: Vec<SearchExplainItem>,
}
