use serde::{Deserialize, Serialize};

use cortex_engine::{AnswerGroundingReport, ContextPipelineTrace};

use super::verification::VerificationReportResponse;
use super::{AnswerGroundingReportResponse, AnswerGroundingSpanResponse};

#[derive(Serialize, Debug, Clone)]
pub struct ScoreComponentResponse {
    pub name: String,
    pub value: u32,
    pub contribution: i32,
    pub reason: String,
}

#[derive(Serialize, Debug, Clone)]
pub struct ExplainResponse {
    pub score: u32,
    pub matched_terms: Vec<String>,
    pub why_selected: String,
    pub score_components: Vec<ScoreComponentResponse>,
    pub base_bm25: u32,
    pub source_trust_q16: u16,
    pub source_trust_category: String,
    pub source_trust_bonus: u32,
    pub source_freshness_q16: u16,
    pub source_freshness_category: String,
    pub source_freshness_bonus: u32,
    pub redundancy_penalty: u32,
}

#[derive(Serialize, Debug, Clone)]
pub struct SourceRefResponse {
    pub source_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_url: Option<String>,
    pub document_id: Option<String>,
    pub page: Option<u32>,
    pub row: Option<u32>,
    pub cell_range: Option<String>,
    pub json_path: Option<String>,
    pub confidence_q16: u16,
}

#[derive(Serialize, Debug, Clone)]
pub struct ContextPackCellResponse {
    pub cell_id: u64,
    pub estimated_tokens: u32,
    pub citation: Option<String>,
    pub payload_text: String,
    pub explain: Option<ExplainResponse>,
    pub source_ref: Option<SourceRefResponse>,
    pub provenance: Option<ContextSpanProvenanceResponse>,
    pub access_decision: Option<ContextAccessDecisionResponse>,
}

#[derive(Serialize, Debug, Clone)]
pub struct ContextAccessDecisionResponse {
    pub cell_id: u64,
    pub decision: String,
    pub policy: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub policy_version: Option<String>,
    pub reason: String,
    pub scope: String,
    pub scope_id: u64,
    pub agent_id: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_view_digest: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub principal_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth_role: Option<String>,
}

#[derive(Serialize, Debug, Clone)]
pub struct ContextSpanProvenanceResponse {
    pub source_cell_id: u64,
    pub source_byte_start: usize,
    pub source_byte_end: usize,
    pub source_line_start: u32,
    pub source_line_end: u32,
    pub source_ref: Option<SourceRefResponse>,
}

#[derive(Serialize, Debug, Clone)]
pub struct ContextPackAnomalyResponse {
    pub cell_id: Option<u64>,
    pub code: String,
    pub message: String,
    pub why_excluded: Option<String>,
}

#[derive(Serialize, Debug, Clone)]
pub struct ContextPackResponse {
    pub schema_version: &'static str,
    pub token_budget_tokens: u32,
    pub estimated_tokens: u32,
    pub truncated: bool,
    pub citations_required: bool,
    pub answerability_q16: u16,
    pub conflict_visibility_q16: u16,
    pub visible_conflict_count: u32,
    pub cells: Vec<ContextPackCellResponse>,
    pub anomalies: Vec<ContextPackAnomalyResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grounding_report: Option<AnswerGroundingReportResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accountability_receipt: Option<serde_json::Value>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ContextTraceRequest {
    pub retrieve_aql: String,
    pub verify_aql: Option<String>,
}

#[derive(Serialize, Debug, Clone)]
pub struct ContextTraceResponse {
    pub schema_version: &'static str,
    pub context: ContextPackResponse,
    pub verification: Option<VerificationReportResponse>,
    pub trace: ContextPipelineTrace,
}

pub(crate) fn map_answer_grounding_report(
    report: &AnswerGroundingReport,
) -> AnswerGroundingReportResponse {
    AnswerGroundingReportResponse {
        answer_supported: report.answer_supported,
        rejected: report.rejected,
        support_q16: report.support_q16,
        supported_span_count: report.supported_span_count,
        unsupported_span_count: report.unsupported_span_count,
        spans: report
            .spans
            .iter()
            .map(|span| AnswerGroundingSpanResponse {
                text: span.text.clone(),
                start_byte: span.start_byte,
                end_byte: span.end_byte,
                support_q16: span.support_q16,
                supported: span.supported,
                covered_terms: span.covered_terms.clone(),
                missing_terms: span.missing_terms.clone(),
                supported_by_cell_ids: span
                    .supported_by_cell_ids
                    .iter()
                    .map(|cell_id| cell_id.0)
                    .collect(),
                citations: span.citations.clone(),
            })
            .collect(),
    }
}
