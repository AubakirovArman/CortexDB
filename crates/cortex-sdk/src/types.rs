use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    NotFound,
    BadRequest,
    Unauthorized,
    Forbidden,
    PayloadTooLarge,
    RateLimited,
    ServiceUnavailable,
    Internal,
    InvalidAql,
    UnknownField,
    UnsupportedOperator,
    PermissionDenied,
    DatabaseBusy,
    StorageCorruption,
    InvalidTenant,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct ErrorResponse {
    pub code: ErrorCode,
    pub error: String,
    pub message: String,
}

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

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub server_version: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct StatsResponse {
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

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct ValidationResponse {
    pub ok: bool,
    pub manifest_ok: bool,
    pub wal_ok: bool,
    pub live_segments_checked: usize,
    pub bitmap_indexes_checked: usize,
    pub lexical_indexes_checked: usize,
    pub vector_indexes_checked: usize,
    pub hnsw_graphs_checked: usize,
    pub cells_checked: usize,
    pub wal_records_checked: u64,
    pub wal_safe_truncate_offset: u64,
    pub errors: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct PutCellResponse {
    pub seq: u64,
    pub cell_id: u64,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct CellResponse {
    pub cell_id: u64,
    pub payload: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct CellLookupResponse {
    pub cell: Option<CellResponse>,
}

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
pub struct AqlExplainResponse {
    pub task: String,
    pub brain_id: u64,
    pub selected_mode: String,
    pub bitmap_plan: String,
    #[serde(default)]
    pub bitmap_ops: Vec<String>,
    #[serde(default)]
    pub filters: Vec<AqlExplainFilter>,
    pub candidate_counts: AqlCandidateCounts,
    pub candidate_limit: u32,
    pub budget_tokens: u32,
    pub citations_required: bool,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct AqlResponse {
    #[serde(default)]
    pub cells: Vec<AqlCellResponse>,
    #[serde(default)]
    pub explain: Option<AqlExplainResponse>,
}

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
    pub source_trust_bonus: u32,
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
    pub document_id: Option<String>,
    pub page: Option<u32>,
    pub cell_range: Option<String>,
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
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct EvidenceResponse {
    pub cell_id: u64,
    pub matched_terms: u32,
    pub source_trust_q16: u16,
    #[serde(default)]
    pub source_trust_category: String,
    pub citation: Option<String>,
    pub payload_text: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct GuardResponse {
    pub cell_id: Option<u64>,
    pub code: String,
    pub message: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct NumericConflictResponse {
    pub metric: String,
    pub left: String,
    pub right: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct VerificationReportResponse {
    pub fact: String,
    pub status: String,
    pub verdict: String,
    pub evidence: Vec<EvidenceResponse>,
    pub contradicting_evidence: Vec<EvidenceResponse>,
    pub guards: Vec<GuardResponse>,
    pub supporting: Vec<EvidenceResponse>,
    pub contradicting: Vec<EvidenceResponse>,
    pub numeric_conflicts: Vec<NumericConflictResponse>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct IngestResponse {
    pub rows_ingested: usize,
    pub chunks_ingested: usize,
    pub facts_ingested: usize,
    pub first_cell_id: Option<u64>,
    pub job_id: Option<u64>,
    pub validation_report: IngestionValidationReport,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct IngestionValidationReport {
    pub cells_seen: usize,
    pub warnings: Vec<IngestionValidationIssue>,
    pub skipped_items: Vec<IngestionSkippedItem>,
    pub source_refs: Vec<IngestionSourceRefReport>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct IngestionValidationIssue {
    pub code: String,
    pub message: String,
    pub cell_id: Option<u64>,
    pub chunk_id: Option<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct IngestionSkippedItem {
    pub reason: String,
    pub input_ref: Option<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct IngestionSourceRefReport {
    pub cell_id: u64,
    pub chunk_id: Option<String>,
    pub has_source_ref: bool,
    pub source_id: Option<String>,
    pub document_id: Option<String>,
    pub confidence_q16: Option<u16>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct RememberResponse {
    pub seq: u64,
    pub cell_id: u64,
    pub ttl_seconds: Option<u64>,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum IngestionJobStatus {
    Queued,
    Running,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct IngestionJobResponse {
    pub job_id: u64,
    pub label: String,
    pub status: IngestionJobStatus,
    pub total_items: Option<u64>,
    pub completed_items: u64,
    pub failed_items: u64,
    pub last_cell_id: Option<u64>,
    pub message: Option<String>,
    #[serde(default)]
    pub retry_count: u32,
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct DeleteJobResponse {
    pub deleted: bool,
}

fn default_max_retries() -> u32 {
    3
}
