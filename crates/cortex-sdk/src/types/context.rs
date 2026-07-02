use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct ExplainResponse {
    pub score: u32,
    pub matched_terms: Vec<String>,
    pub why_selected: String,
    #[serde(default)]
    pub score_components: Vec<ScoreComponentResponse>,
    pub base_bm25: u32,
    #[serde(default)]
    pub source_trust_q16: u16,
    #[serde(default)]
    pub source_trust_category: String,
    #[serde(default)]
    pub source_trust_bonus: u32,
    #[serde(default)]
    pub source_freshness_q16: u16,
    #[serde(default)]
    pub source_freshness_category: String,
    #[serde(default)]
    pub source_freshness_bonus: u32,
    #[serde(default)]
    pub redundancy_penalty: u32,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct ScoreComponentResponse {
    pub name: String,
    pub value: u32,
    pub contribution: i32,
    pub reason: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct SourceRefResponse {
    pub source_id: String,
    #[serde(default)]
    pub source_url: Option<String>,
    #[serde(default)]
    pub document_id: Option<String>,
    #[serde(default)]
    pub page: Option<u32>,
    #[serde(default)]
    pub row: Option<u32>,
    #[serde(default)]
    pub cell_range: Option<String>,
    #[serde(default)]
    pub json_path: Option<String>,
    pub confidence_q16: u16,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct ContextPackCellResponse {
    pub cell_id: u64,
    pub estimated_tokens: u32,
    pub citation: Option<String>,
    pub payload_text: String,
    pub explain: Option<ExplainResponse>,
    pub source_ref: Option<SourceRefResponse>,
    #[serde(default)]
    pub provenance: Option<ContextSpanProvenanceResponse>,
    #[serde(default)]
    pub access_decision: Option<ContextAccessDecisionResponse>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct ContextAccessDecisionResponse {
    pub cell_id: u64,
    pub decision: String,
    pub policy: String,
    #[serde(default)]
    pub policy_version: Option<String>,
    pub reason: String,
    pub scope: String,
    pub scope_id: u64,
    #[serde(default)]
    pub agent_id: Option<u64>,
    #[serde(default)]
    pub agent_view_digest: Option<String>,
    #[serde(default)]
    pub principal_id: Option<String>,
    #[serde(default)]
    pub auth_role: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct ContextSpanProvenanceResponse {
    pub source_cell_id: u64,
    pub source_byte_start: usize,
    pub source_byte_end: usize,
    pub source_line_start: u32,
    pub source_line_end: u32,
    #[serde(default)]
    pub source_ref: Option<SourceRefResponse>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct ContextPackAnomalyResponse {
    pub cell_id: Option<u64>,
    pub code: String,
    pub message: String,
    #[serde(default)]
    pub why_excluded: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct ContextPackResponse {
    pub schema_version: String,
    pub token_budget_tokens: u32,
    pub estimated_tokens: u32,
    pub truncated: bool,
    pub citations_required: bool,
    pub answerability_q16: u16,
    pub conflict_visibility_q16: u16,
    pub visible_conflict_count: u32,
    pub cells: Vec<ContextPackCellResponse>,
    pub anomalies: Vec<ContextPackAnomalyResponse>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grounding_report: Option<AnswerGroundingReportResponse>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accountability_receipt: Option<serde_json::Value>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct AnswerGroundingOptionsResponse {
    pub min_span_support_q16: u16,
    pub require_citations: bool,
    pub reject_unsupported: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
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

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct AnswerGroundingReportResponse {
    pub answer_supported: bool,
    pub rejected: bool,
    pub support_q16: u16,
    pub supported_span_count: u32,
    pub unsupported_span_count: u32,
    pub spans: Vec<AnswerGroundingSpanResponse>,
}
