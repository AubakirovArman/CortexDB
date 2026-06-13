pub use cortex_api_types::search::{
    AnnEvaluationResponse, AnnNoFallbackDecisionResponse, AnnSearchReportResponse,
    AnswerGroundingReportResponse, AnswerGroundingSpanResponse, HnswNoFallbackProfileResponse,
    SearchExplainItemResponse, SearchExplainResponse, SearchExplainTermContributionResponse,
    SearchResponse, SearchResultResponse, SearchRoutingDecisionResponse,
};

use serde::Serialize;

#[derive(Serialize, Debug, Clone)]
pub struct LlmInferenceAuditResponse {
    pub context_pack_only: bool,
    pub prompt_body_logged: bool,
    pub secrets_logged: bool,
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
