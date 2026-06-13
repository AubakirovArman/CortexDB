use serde::Serialize;

mod context;

pub use context::*;

#[derive(Serialize)]
pub struct ErrorResponse {
    pub code: String,
    pub error: String,
    pub message: String,
}

#[derive(Serialize)]
pub struct CliStatsResponse {
    pub current_seq: u64,
    pub checkpoint_seq: u64,
    pub live_segments: usize,
    pub retired_segments: usize,
    pub memtable_cells: usize,
    pub memtable_versions: usize,
    pub memtable_payload_bytes: usize,
    pub estimated_memtable_bytes: usize,
    pub estimated_index_bytes: usize,
    pub estimated_context_pack_bytes: usize,
    pub estimated_total_memory_bytes: usize,
    pub live_segment_bytes: u64,
    pub retired_segment_bytes: u64,
    pub total_segment_bytes: u64,
    pub durable_storage_bytes: u64,
    pub live_segment_payload_bytes: u64,
    pub logical_payload_bytes: u64,
    pub space_amplification_q16: u32,
    pub write_amplification_q16: u32,
    pub compaction_pressure_q16: u32,
    pub wal_size_bytes: u64,
    pub wal_writer_records: u64,
    pub wal_writer_bytes: u64,
    pub wal_writer_fsyncs: u64,
    pub wal_writer_batches: u64,
}

#[derive(Serialize)]
pub struct CliValidateResponse {
    pub ok: bool,
    pub live_segments_checked: usize,
    pub cells_checked: usize,
    pub wal_records_checked: u64,
    pub wal_safe_truncate_offset: u64,
    pub issue_count: usize,
    pub issues: Vec<CliValidationIssueResponse>,
}

#[derive(Serialize)]
pub struct CliValidationIssueResponse {
    pub kind: String,
    pub message: String,
    pub recovery_action: String,
    pub recommended_command: String,
    pub requires_restore: bool,
}

#[derive(Serialize)]
pub struct CliAnnValidateResponse {
    pub ok: bool,
    pub vector_indexes_checked: usize,
    pub hnsw_graphs_checked: usize,
    pub errors: Vec<String>,
}

#[derive(Serialize)]
pub struct CliVectorRebuildResponse {
    pub segments_checked: usize,
    pub cells_scanned: usize,
    pub vector_candidates: usize,
    pub vector_indexes_rebuilt: usize,
    pub hnsw_graphs_rebuilt: usize,
    pub hnsw_enabled: bool,
}

#[derive(Serialize)]
pub struct CliAnnSearchReportResponse {
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

#[derive(Serialize)]
pub struct CliNoFallbackDecisionResponse {
    pub allowed: bool,
    pub reasons: Vec<String>,
}

#[derive(Serialize)]
pub struct CliNoFallbackProfileResponse {
    pub configured: bool,
    pub rollout_enabled: Option<bool>,
    pub min_recall_q16: Option<u16>,
    pub require_upper_layers: Option<bool>,
}

#[derive(Serialize)]
pub struct CliAnnEvaluationResponse {
    pub available: bool,
    pub reason: Option<String>,
    pub ann_report: Option<CliAnnSearchReportResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub no_fallback_decision: Option<CliNoFallbackDecisionResponse>,
    pub exact_top_k: Vec<u32>,
    pub ann_top_k: Vec<u32>,
    pub overlap_count: usize,
    pub recall_q16: u16,
}

#[derive(Serialize)]
pub struct VerificationResponse {
    pub verdict: String,
    pub supporting: Vec<VerificationEvidenceResponse>,
    pub contradicting: Vec<VerificationEvidenceResponse>,
    pub numeric_conflicts: Vec<NumericConflictResponse>,
}

#[derive(Serialize)]
pub struct VerificationEvidenceResponse {
    pub cell_id: u64,
    pub matched_terms: u32,
    pub match_score_q16: u16,
    pub match_kind: String,
    pub source_trust_q16: u16,
    pub source_trust_category: String,
    pub citation: Option<String>,
    pub payload_text: String,
}

#[derive(Serialize)]
pub struct NumericConflictResponse {
    pub metric: String,
    pub left: String,
    pub right: String,
}

#[derive(Serialize)]
pub struct CellResponse {
    pub cell_id: u64,
    pub seq: u64,
    pub payload: String,
}

#[derive(Serialize)]
pub struct AqlCellResponse {
    pub cell_id: u64,
    pub payload: String,
}

#[derive(Serialize)]
pub struct AqlExplainFilterResponse {
    pub kind: String,
    pub expression: String,
}

#[derive(Serialize)]
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

#[derive(Serialize)]
pub struct AqlCostModelTermResponse {
    pub term: String,
    pub document_frequency: u64,
}

#[derive(Serialize)]
pub struct AqlCostModelEstimateResponse {
    pub path: String,
    pub cost_units: u64,
}

#[derive(Serialize)]
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

#[derive(Serialize)]
pub struct AqlLogicalPlanNodeResponse {
    pub id: usize,
    pub kind: String,
    pub detail: String,
    pub permission_predicate: Option<String>,
}

#[derive(Serialize)]
pub struct AqlLogicalPlanResponse {
    pub nodes: Vec<AqlLogicalPlanNodeResponse>,
    pub policy_complete: bool,
}

#[derive(Serialize)]
pub struct AqlExecutionOperatorResponse {
    pub name: String,
    pub input_count: usize,
    pub output_count: usize,
    pub elapsed_nanos: u64,
}

#[derive(Serialize)]
pub struct AqlExecutionTraceResponse {
    pub operators: Vec<AqlExecutionOperatorResponse>,
    pub total_elapsed_nanos: u64,
}

#[derive(Serialize)]
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

#[derive(Serialize)]
pub struct AqlResponse {
    pub cells: Vec<AqlCellResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub explain: Option<AqlExplainResponse>,
}

#[derive(Serialize)]
pub struct SearchResultResponse {
    pub cell_id: u64,
    pub score: u64,
    pub lexical_score: u64,
    pub vector_score: u64,
    pub payload: String,
}

#[derive(Serialize)]
pub struct SearchResponse {
    pub search_mode: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub routing: Option<SearchRoutingDecisionResponse>,
    pub results: Vec<SearchResultResponse>,
}

#[derive(Serialize)]
pub struct SearchRoutingDecisionResponse {
    pub requested_mode: String,
    pub selected_strategy: String,
    pub reason: String,
    pub text_available: bool,
    pub vector_available: bool,
}

#[derive(Serialize)]
pub struct RememberResponse {
    pub seq: u64,
    pub cell_id: u64,
    pub ttl_seconds: Option<u64>,
}
