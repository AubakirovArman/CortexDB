use serde::Deserialize;

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct AqlCellResponse {
    pub cell_id: u64,
    pub payload: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct AqlExplainFilter {
    pub kind: String,
    pub expression: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct AqlCandidateCounts {
    pub universe: usize,
    pub agent_allowed: usize,
    pub live: usize,
    pub after_bitmap: usize,
    pub after_quality: usize,
    pub returned_limit: usize,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct AqlLogicalPlanNode {
    pub id: usize,
    pub kind: String,
    pub detail: String,
    #[serde(default)]
    pub permission_predicate: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq)]
pub struct AqlLogicalPlan {
    #[serde(default)]
    pub nodes: Vec<AqlLogicalPlanNode>,
    #[serde(default)]
    pub policy_complete: bool,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct AqlExecutionOperator {
    pub name: String,
    pub input_count: usize,
    pub output_count: usize,
    pub elapsed_nanos: u64,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct AqlExecutionTrace {
    #[serde(default)]
    pub operators: Vec<AqlExecutionOperator>,
    pub total_elapsed_nanos: u64,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct AqlExplainResponse {
    pub task: String,
    pub brain_id: u64,
    pub selected_mode: String,
    #[serde(default)]
    pub logical_plan: AqlLogicalPlan,
    #[serde(default)]
    pub policy_rewritten_plan: AqlLogicalPlan,
    pub bitmap_plan: String,
    #[serde(default)]
    pub bitmap_ops: Vec<String>,
    #[serde(default)]
    pub filters: Vec<AqlExplainFilter>,
    pub candidate_counts: AqlCandidateCounts,
    pub candidate_limit: u32,
    pub budget_tokens: u32,
    pub citations_required: bool,
    #[serde(default)]
    pub execution_trace: Option<AqlExecutionTrace>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct AqlResponse {
    #[serde(default)]
    pub cells: Vec<AqlCellResponse>,
    #[serde(default)]
    pub explain: Option<AqlExplainResponse>,
}
