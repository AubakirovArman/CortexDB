use serde::Serialize;

#[derive(Serialize, Debug, Clone)]
pub struct AqlCellResponse {
    pub cell_id: u64,
    pub payload: String,
}

#[derive(Serialize, Debug, Clone)]
pub struct AqlExplainFilterResponse {
    pub kind: String,
    pub expression: String,
}

#[derive(Serialize, Debug, Clone)]
pub struct AqlCandidateCountsResponse {
    pub universe: usize,
    pub agent_allowed: usize,
    pub live: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub estimated_after_bitmap: Option<usize>,
    pub after_bitmap: usize,
    pub after_quality: usize,
    pub returned_limit: usize,
}

#[derive(Serialize, Debug, Clone)]
pub struct AqlCostModelTermResponse {
    pub term: String,
    pub document_frequency: u64,
}

#[derive(Serialize, Debug, Clone)]
pub struct AqlCostModelEstimateResponse {
    pub path: String,
    pub cost_units: u64,
}

#[derive(Serialize, Debug, Clone)]
pub struct AqlCostModelResponse {
    pub selected_path: String,
    pub reason: String,
    pub estimated_live_rows: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub estimated_after_bitmap: Option<u64>,
    pub recommended_candidate_limit: u32,
    pub has_query_vector: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rarest_term: Option<AqlCostModelTermResponse>,
    pub estimates: Vec<AqlCostModelEstimateResponse>,
}

#[derive(Serialize, Debug, Clone)]
pub struct AqlLogicalPlanNodeResponse {
    pub id: usize,
    pub kind: String,
    pub detail: String,
    pub permission_predicate: Option<String>,
}

#[derive(Serialize, Debug, Clone)]
pub struct AqlLogicalPlanResponse {
    pub nodes: Vec<AqlLogicalPlanNodeResponse>,
    pub policy_complete: bool,
}

#[derive(Serialize, Debug, Clone)]
pub struct AqlExecutionOperatorResponse {
    pub name: String,
    pub input_count: usize,
    pub output_count: usize,
    pub elapsed_nanos: u64,
}

#[derive(Serialize, Debug, Clone)]
pub struct AqlExecutionTraceResponse {
    pub operators: Vec<AqlExecutionOperatorResponse>,
    pub total_elapsed_nanos: u64,
}

#[derive(Serialize, Debug, Clone)]
pub struct AqlExplainResponse {
    pub task: String,
    pub brain_id: u64,
    pub selected_mode: String,
    pub logical_plan: AqlLogicalPlanResponse,
    pub policy_rewritten_plan: AqlLogicalPlanResponse,
    pub bitmap_plan: String,
    pub bitmap_ops: Vec<String>,
    pub filters: Vec<AqlExplainFilterResponse>,
    pub cost_model: AqlCostModelResponse,
    pub candidate_counts: AqlCandidateCountsResponse,
    pub candidate_limit: u32,
    pub budget_tokens: u32,
    pub citations_required: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution_trace: Option<AqlExecutionTraceResponse>,
}

#[derive(Serialize, Debug, Clone)]
pub struct AqlResponse {
    pub cells: Vec<AqlCellResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub explain: Option<AqlExplainResponse>,
}

#[derive(Serialize, Debug, Clone)]
pub struct RememberResponse {
    pub seq: u64,
    pub cell_id: u64,
    pub ttl_seconds: Option<u64>,
}
