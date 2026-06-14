use serde::{Deserialize, Serialize};

pub use crate::core::RememberResponse;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct AqlCellResponse {
    pub cell_id: u64,
    pub payload: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct AqlExplainFilterResponse {
    pub kind: String,
    pub expression: String,
}

pub type AqlExplainFilter = AqlExplainFilterResponse;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct AqlCandidateCountsResponse {
    pub universe: usize,
    pub agent_allowed: usize,
    pub live: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub estimated_after_bitmap: Option<usize>,
    pub after_bitmap: usize,
    pub after_quality: usize,
    pub returned_limit: usize,
}

pub type AqlCandidateCounts = AqlCandidateCountsResponse;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct AqlCostModelTermResponse {
    pub term: String,
    pub document_frequency: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct AqlCostModelEstimateResponse {
    pub path: String,
    pub cost_units: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct AqlCostModelResponse {
    pub selected_path: String,
    pub reason: String,
    pub estimated_live_rows: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub estimated_after_bitmap: Option<u64>,
    pub recommended_candidate_limit: u32,
    pub has_query_vector: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rarest_term: Option<AqlCostModelTermResponse>,
    #[serde(default)]
    pub estimates: Vec<AqlCostModelEstimateResponse>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct AqlLogicalPlanNodeResponse {
    pub id: usize,
    pub kind: String,
    pub detail: String,
    #[serde(default)]
    pub permission_predicate: Option<String>,
}

pub type AqlLogicalPlanNode = AqlLogicalPlanNodeResponse;

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct AqlLogicalPlanResponse {
    #[serde(default)]
    pub nodes: Vec<AqlLogicalPlanNodeResponse>,
    #[serde(default)]
    pub policy_complete: bool,
}

pub type AqlLogicalPlan = AqlLogicalPlanResponse;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct AqlExecutionOperatorResponse {
    pub name: String,
    pub input_count: usize,
    pub output_count: usize,
    #[serde(default)]
    pub actual_input_count: usize,
    #[serde(default)]
    pub actual_output_count: usize,
    #[serde(default)]
    pub estimated_output_count: Option<usize>,
    pub elapsed_nanos: u64,
}

pub type AqlExecutionOperator = AqlExecutionOperatorResponse;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct AqlExecutionTraceResponse {
    #[serde(default)]
    pub operators: Vec<AqlExecutionOperatorResponse>,
    pub total_elapsed_nanos: u64,
}

pub type AqlExecutionTrace = AqlExecutionTraceResponse;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct AqlExplainResponse {
    pub task: String,
    pub brain_id: u64,
    pub selected_mode: String,
    #[serde(default)]
    pub logical_plan: AqlLogicalPlanResponse,
    #[serde(default)]
    pub policy_rewritten_plan: AqlLogicalPlanResponse,
    pub bitmap_plan: String,
    #[serde(default)]
    pub bitmap_ops: Vec<String>,
    #[serde(default)]
    pub filters: Vec<AqlExplainFilterResponse>,
    #[serde(default)]
    pub cost_model: Option<AqlCostModelResponse>,
    pub candidate_counts: AqlCandidateCountsResponse,
    pub candidate_limit: u32,
    pub budget_tokens: u32,
    pub citations_required: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub execution_trace: Option<AqlExecutionTraceResponse>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct AqlResponse {
    #[serde(default)]
    pub cells: Vec<AqlCellResponse>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub explain: Option<AqlExplainResponse>,
}
