use serde::Serialize;

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
}

#[derive(Serialize)]
pub struct ContextPackCellResponse {
    pub cell_id: u64,
    pub estimated_tokens: u32,
    pub citation: Option<String>,
    pub payload_text: String,
    pub explain: Option<ContextPackExplainResponse>,
    pub source_ref: Option<SourceRefResponse>,
}

#[derive(Serialize)]
pub struct ContextPackExplainResponse {
    pub score: u32,
    pub matched_terms: Vec<String>,
    pub why_selected: String,
    pub score_components: Vec<ContextPackScoreComponentResponse>,
    pub base_bm25: u32,
    pub source_trust_q16: u16,
    pub source_trust_category: String,
    pub source_trust_bonus: u32,
    pub redundancy_penalty: u32,
}

#[derive(Serialize)]
pub struct ContextPackScoreComponentResponse {
    pub name: String,
    pub value: u32,
    pub contribution: i32,
    pub reason: String,
}

#[derive(Serialize)]
pub struct SourceRefResponse {
    pub source_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_url: Option<String>,
    pub document_id: Option<String>,
    pub page: Option<u32>,
    pub cell_range: Option<String>,
    pub json_path: Option<String>,
    pub confidence_q16: u16,
}

#[derive(Serialize)]
pub struct ContextPackAnomalyResponse {
    pub cell_id: Option<u64>,
    pub code: String,
    pub message: String,
    pub why_excluded: Option<String>,
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
    pub after_bitmap: usize,
    pub after_quality: usize,
    pub returned_limit: usize,
}

#[derive(Serialize)]
pub struct AqlExplainResponse {
    pub task: String,
    pub brain_id: u64,
    pub selected_mode: String,
    pub bitmap_plan: String,
    pub bitmap_ops: Vec<String>,
    pub filters: Vec<AqlExplainFilterResponse>,
    pub candidate_counts: AqlCandidateCountsResponse,
    pub candidate_limit: u32,
    pub budget_tokens: u32,
    pub citations_required: bool,
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
