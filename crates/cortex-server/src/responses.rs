use serde::{Deserialize, Serialize};

#[derive(Serialize, Debug, Clone)]
pub struct HealthResponse {
    pub status: String,
    /// API version path segment (e.g. "v1").
    pub version: String,
    /// Server package version from Cargo.toml.
    pub server_version: String,
}

#[derive(Serialize, Debug, Clone)]
pub struct ClusterNodeResponse {
    pub id: u64,
    pub address: String,
}

#[derive(Serialize, Debug, Clone)]
pub struct ClusterStatusResponse {
    pub local_node: u64,
    pub nodes: Vec<ClusterNodeResponse>,
    pub replication_factor: usize,
    pub distributed_enabled: bool,
}

/// Response metrics containing detailed storage, MemTable, and WAL statistics.
#[derive(Serialize, Debug, Clone)]
pub struct StatsResponse {
    /// The current global database commit sequence.
    pub current_seq: u64,
    /// The commit sequence of the last successful checkpoint.
    pub checkpoint_seq: u64,
    /// The number of active LSM-segments currently being queried.
    pub live_segments: usize,
    /// The number of garbage-collected or retired segments on disk.
    pub retired_segments: usize,
    /// Total number of unique knowledge cell IDs in MemTable.
    pub memtable_cells: usize,
    /// Total number of cell versions currently held in MemTable.
    pub memtable_versions: usize,
    /// Total size of the active Write-Ahead Log (.aclog) files in bytes.
    pub wal_size_bytes: u64,
    /// Total number of transaction log records appended.
    pub wal_writer_records: u64,
    /// Total bytes appended to the active WAL file.
    pub wal_writer_bytes: u64,
    /// Total number of disk fsync flushes executed by the WAL writer.
    pub wal_writer_fsyncs: u64,
    /// Total number of batches committed under group commit.
    pub wal_writer_batches: u64,
}

/// Validation report containing integrity verification results.
#[derive(Serialize, Debug, Clone)]
pub struct ValidationResponse {
    /// True if there are absolutely zero structural errors or mismatches.
    pub ok: bool,
    /// True if storage manifest is valid.
    pub manifest_ok: bool,
    /// True if WAL records match manifest transactions.
    pub wal_ok: bool,
    pub live_segments_checked: usize,
    pub bitmap_indexes_checked: usize,
    pub lexical_indexes_checked: usize,
    pub vector_indexes_checked: usize,
    pub hnsw_graphs_checked: usize,
    pub cells_checked: usize,
    pub wal_records_checked: u64,
    pub wal_safe_truncate_offset: u64,
    /// List of detected validation errors or warnings.
    pub errors: Vec<String>,
}

#[derive(Serialize, Debug, Clone)]
pub struct CellResponse {
    pub cell_id: u64,
    pub payload: String,
}

#[derive(Serialize, Debug, Clone)]
pub struct CellLookupResponse {
    pub cell: Option<CellResponse>,
}

#[derive(Serialize, Debug, Clone)]
pub struct PutCellResponse {
    pub seq: u64,
    pub cell_id: u64,
}

#[derive(Serialize, Debug, Clone)]
pub struct DeleteJobResponse {
    pub deleted: bool,
}

#[derive(Serialize, Debug, Clone)]
pub struct CheckpointResponse {
    pub checkpoint_seq: u64,
    pub cells_flushed: usize,
}

#[derive(Serialize, Debug, Clone)]
pub struct ExplainResponse {
    pub score: u32,
    pub matched_terms: Vec<String>,
    pub why_selected: String,
    pub base_bm25: u32,
    pub source_trust_bonus: u32,
    pub redundancy_penalty: u32,
}

#[derive(Serialize, Debug, Clone)]
pub struct SourceRefResponse {
    pub source_id: String,
    pub document_id: Option<String>,
    pub page: Option<u32>,
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
}

#[derive(Serialize, Debug, Clone)]
pub struct ContextPackAnomalyResponse {
    pub cell_id: Option<u64>,
    pub code: String,
    pub message: String,
}

#[derive(Serialize, Debug, Clone)]
pub struct ContextPackResponse {
    pub schema_version: &'static str,
    pub token_budget_tokens: u32,
    pub estimated_tokens: u32,
    pub truncated: bool,
    pub citations_required: bool,
    pub cells: Vec<ContextPackCellResponse>,
    pub anomalies: Vec<ContextPackAnomalyResponse>,
}

#[derive(Serialize, Debug, Clone)]
pub struct EvidenceResponse {
    pub cell_id: u64,
    pub matched_terms: u32,
    pub source_trust_q16: u16,
    pub citation: Option<String>,
    pub payload_text: String,
}

#[derive(Serialize, Debug, Clone)]
pub struct GuardResponse {
    pub cell_id: Option<u64>,
    pub code: String,
    pub message: String,
}

#[derive(Serialize, Debug, Clone)]
pub struct NumericConflictResponse {
    pub metric: String,
    pub left: String,
    pub right: String,
}

#[derive(Serialize, Debug, Clone)]
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

#[derive(Serialize, Debug, Clone)]
pub struct SearchResultResponse {
    pub cell_id: u64,
    pub score: u64,
    pub lexical_score: u64,
    pub vector_score: u64,
    pub payload: String,
}

#[derive(Serialize, Debug, Clone)]
pub struct AnnSearchReportResponse {
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

#[derive(Serialize, Debug, Clone)]
pub struct AnnNoFallbackDecisionResponse {
    pub allowed: bool,
    pub reasons: Vec<String>,
}

#[derive(Serialize, Debug, Clone)]
pub struct HnswNoFallbackProfileResponse {
    pub configured: bool,
    pub rollout_enabled: Option<bool>,
    pub min_recall_q16: Option<u16>,
    pub require_upper_layers: Option<bool>,
}

#[derive(Serialize, Debug, Clone)]
pub struct SearchResponse {
    pub search_mode: String,
    pub ann_report: Option<AnnSearchReportResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub no_fallback_decision: Option<AnnNoFallbackDecisionResponse>,
    pub results: Vec<SearchResultResponse>,
}

#[derive(Serialize, Debug, Clone)]
pub struct SearchExplainItemResponse {
    pub cell_id: u64,
    pub score: u64,
    pub lexical_score: u64,
    pub vector_score: u64,
    pub payload_preview: String,
}

#[derive(Serialize, Debug, Clone)]
pub struct SearchExplainResponse {
    pub query_terms: Vec<String>,
    pub search_mode: String,
    pub results: Vec<SearchExplainItemResponse>,
}

#[derive(Serialize, Debug, Clone)]
pub struct AnnEvaluationResponse {
    pub available: bool,
    pub reason: Option<String>,
    pub ann_report: Option<AnnSearchReportResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub no_fallback_decision: Option<AnnNoFallbackDecisionResponse>,
    pub exact_top_k: Vec<u32>,
    pub ann_top_k: Vec<u32>,
    pub overlap_count: usize,
    pub recall_q16: u16,
}

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
    pub audit: LlmInferenceAuditResponse,
}

#[derive(Serialize, Debug, Clone)]
pub struct AqlCellResponse {
    pub cell_id: u64,
    pub payload: String,
}

#[derive(Serialize, Debug, Clone)]
pub struct AqlResponse {
    pub cells: Vec<AqlCellResponse>,
}

#[derive(Serialize, Debug, Clone)]
pub struct RememberResponse {
    pub seq: u64,
    pub cell_id: u64,
    pub ttl_seconds: Option<u64>,
}

#[derive(Serialize, Debug, Clone)]
pub struct IngestResponse {
    pub rows_ingested: usize,
    pub chunks_ingested: usize,
    pub facts_ingested: usize,
    pub first_cell_id: Option<u64>,
    pub job_id: Option<u64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
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
    PermissionDenied,
    DatabaseBusy,
    StorageCorruption,
    InvalidTenant,
}

impl ErrorCode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NotFound => "not_found",
            Self::BadRequest => "bad_request",
            Self::Unauthorized => "unauthorized",
            Self::Forbidden => "forbidden",
            Self::PayloadTooLarge => "payload_too_large",
            Self::RateLimited => "rate_limited",
            Self::ServiceUnavailable => "service_unavailable",
            Self::Internal => "internal",
            Self::InvalidAql => "invalid_aql",
            Self::PermissionDenied => "permission_denied",
            Self::DatabaseBusy => "database_busy",
            Self::StorageCorruption => "storage_corruption",
            Self::InvalidTenant => "invalid_tenant",
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ErrorResponse {
    pub code: ErrorCode,
    pub error: String,
    pub message: String,
}

/// Cumulative latency histogram buckets in milliseconds.
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct LatencyHistogramResponse {
    pub count: u64,
    pub sum_ms: u64,
    pub le_10_ms: u64,
    pub le_50_ms: u64,
    pub le_100_ms: u64,
    pub le_500_ms: u64,
    pub le_1000_ms: u64,
    pub gt_1000_ms: u64,
}

/// Aggregated metrics combining storage, WAL, MemTable, and ANN/HNSW stats.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MetricsResponse {
    pub current_seq: u64,
    pub checkpoint_seq: u64,
    pub live_segments: usize,
    pub retired_segments: usize,
    pub memtable_cells: usize,
    pub memtable_versions: usize,
    pub wal_size_bytes: u64,
    pub wal_writer_records: u64,
    pub wal_writer_bytes: u64,
    pub wal_writer_fsyncs: u64,
    pub wal_writer_batches: u64,
    pub ann_graph_nodes: usize,
    pub ann_total_edges: usize,
    pub ann_persisted_segments: usize,
    pub ann_has_checkpoint: bool,
    pub ann_has_uncheckpointed_changes: bool,
    pub ann_search_requests: u64,
    pub ann_fallbacks: u64,
    pub ann_no_fallback_requests: u64,
    pub ann_no_fallback_allowed: u64,
    pub ann_no_fallback_blocked: u64,
    pub ann_search_latency_ms: LatencyHistogramResponse,
    pub actor_queue_depth: usize,
    pub actor_queue_capacity: usize,
    pub request_count: u64,
    pub request_rejected: u64,
    pub request_duration_ms_total: u64,
    pub validation_failures: u64,
}

/// Typed router error taxonomy for consistent HTTP status mapping.
#[derive(Serialize, Debug, Clone)]
pub struct AnnMetricsResponse {
    pub graph_nodes: usize,
    pub total_edges: usize,
    pub persisted_segments: usize,
    pub has_checkpoint: bool,
    pub has_uncheckpointed_changes: bool,
    pub deleted_vectors: usize,
    pub rebuild_count: u64,
}

#[derive(Debug)]
pub enum RouterError {
    NotFound(String),
    BadRequest(String),
    InvalidAql(String),
    PermissionDenied(String),
    Unauthorized,
    Forbidden(String),
    PayloadTooLarge,
    RateLimited,
    DatabaseBusy(String),
    ServiceUnavailable,
    StorageCorruption(String),
    Internal(String),
}

impl RouterError {
    pub fn code(&self) -> ErrorCode {
        match self {
            Self::NotFound(_) => ErrorCode::NotFound,
            Self::BadRequest(_) => ErrorCode::BadRequest,
            Self::InvalidAql(_) => ErrorCode::InvalidAql,
            Self::PermissionDenied(_) => ErrorCode::PermissionDenied,
            Self::Unauthorized => ErrorCode::Unauthorized,
            Self::Forbidden(_) => ErrorCode::Forbidden,
            Self::PayloadTooLarge => ErrorCode::PayloadTooLarge,
            Self::RateLimited => ErrorCode::RateLimited,
            Self::DatabaseBusy(_) => ErrorCode::DatabaseBusy,
            Self::ServiceUnavailable => ErrorCode::ServiceUnavailable,
            Self::StorageCorruption(_) => ErrorCode::StorageCorruption,
            Self::Internal(_) => ErrorCode::Internal,
        }
    }
}

impl std::fmt::Display for RouterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RouterError::NotFound(msg) => write!(f, "{msg}"),
            RouterError::BadRequest(msg) => write!(f, "{msg}"),
            RouterError::InvalidAql(msg) => write!(f, "{msg}"),
            RouterError::PermissionDenied(msg) => write!(f, "{msg}"),
            RouterError::Unauthorized => write!(f, "missing or invalid authorization"),
            RouterError::Forbidden(msg) => write!(f, "{msg}"),
            RouterError::PayloadTooLarge => write!(f, "request body exceeds 2MB limit"),
            RouterError::RateLimited => write!(f, "request rate limit exceeded"),
            RouterError::DatabaseBusy(msg) => write!(f, "{msg}"),
            RouterError::ServiceUnavailable => write!(f, "database actor busy"),
            RouterError::StorageCorruption(msg) => write!(f, "{msg}"),
            RouterError::Internal(msg) => write!(f, "{msg}"),
        }
    }
}

impl From<String> for RouterError {
    fn from(value: String) -> Self {
        match value.as_str() {
            "cell not found" | "job not found" => RouterError::NotFound(value),
            _ => RouterError::BadRequest(value),
        }
    }
}

impl From<std::io::Error> for RouterError {
    fn from(value: std::io::Error) -> Self {
        RouterError::Internal(value.to_string())
    }
}

impl From<serde_json::Error> for RouterError {
    fn from(value: serde_json::Error) -> Self {
        RouterError::Internal(value.to_string())
    }
}

impl From<cortex_engine::error::EngineError> for RouterError {
    fn from(e: cortex_engine::error::EngineError) -> Self {
        use cortex_aql::BindError;
        use cortex_engine::error::EngineError;
        let msg = e.to_string();
        match e {
            EngineError::AqlParse(_) => RouterError::InvalidAql(msg),
            EngineError::AqlBind(BindError::PolicyDenied(error)) => {
                RouterError::PermissionDenied(error.safe_message().to_owned())
            }
            EngineError::AqlBind(error) => RouterError::InvalidAql(error.safe_message().to_owned()),
            EngineError::InvalidOperation => RouterError::BadRequest(msg),
            EngineError::DatabaseAlreadyOpen(_) => RouterError::DatabaseBusy(msg),
            EngineError::Storage(_) => RouterError::StorageCorruption(msg),
            EngineError::StorageInvariant(_)
            | EngineError::MissingStorageFile(_)
            | EngineError::FatalCellMissingAfterWal(_)
            | EngineError::MissingWalSection(_)
            | EngineError::MissingCommitSeq => RouterError::StorageCorruption(msg),
            _ => RouterError::Internal(msg),
        }
    }
}

impl RouterError {
    pub fn status_code(&self) -> u16 {
        match self {
            RouterError::NotFound(_) => 404,
            RouterError::BadRequest(_) => 400,
            RouterError::InvalidAql(_) => 400,
            RouterError::PermissionDenied(_) => 403,
            RouterError::Unauthorized => 401,
            RouterError::Forbidden(_) => 403,
            RouterError::PayloadTooLarge => 413,
            RouterError::RateLimited => 429,
            RouterError::DatabaseBusy(_) => 503,
            RouterError::ServiceUnavailable => 503,
            RouterError::StorageCorruption(_) => 500,
            RouterError::Internal(_) => 500,
        }
    }
}
