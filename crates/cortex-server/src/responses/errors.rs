use cortex_api_types::core::AqlQueryCacheStatsResponse;
use serde::{Deserialize, Serialize};

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
    UnknownField,
    UnsupportedOperator,
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
            Self::UnknownField => "unknown_field",
            Self::UnsupportedOperator => "unsupported_operator",
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
    pub aql_query_cache: AqlQueryCacheStatsResponse,
    pub backup_latest_age_seconds: i64,
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
    pub actor_queue_wait_latency_ms: LatencyHistogramResponse,
    pub actor_queue_wait_p95_ms: u64,
    pub active_readers: usize,
    pub waiting_writers: usize,
    pub request_count: u64,
    pub request_rejected: u64,
    pub request_duration_ms_total: u64,
    pub request_id_client_provided: u64,
    pub request_id_generated: u64,
    pub validation_failures: u64,
    pub principal_quota_requests_allowed: u64,
    pub principal_quota_requests_rejected: u64,
    pub principal_quota_body_bytes_allowed: u64,
    pub principal_quota_body_bytes_rejected: u64,
    pub principal_quota_queue_acquired: u64,
    pub principal_quota_queue_rejected: u64,
    pub compactions_triggered: u64,
    pub compactions_completed: u64,
    pub compaction_duration_ms_total: u64,
    pub compaction_cells_compacted: u64,
    pub compaction_input_bytes: u64,
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
    UnknownField(String),
    UnsupportedOperator(String),
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
            Self::UnknownField(_) => ErrorCode::UnknownField,
            Self::UnsupportedOperator(_) => ErrorCode::UnsupportedOperator,
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
            RouterError::UnknownField(msg) => write!(f, "{msg}"),
            RouterError::UnsupportedOperator(msg) => write!(f, "{msg}"),
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

impl From<cortex_engine::EngineError> for RouterError {
    fn from(e: cortex_engine::EngineError) -> Self {
        use cortex_engine::EngineErrorCode;
        let msg = e.safe_message();
        match e.code() {
            EngineErrorCode::BadRequest => RouterError::BadRequest(msg),
            EngineErrorCode::InvalidAql => RouterError::InvalidAql(msg),
            EngineErrorCode::UnknownField => RouterError::UnknownField(msg),
            EngineErrorCode::UnsupportedOperator => RouterError::UnsupportedOperator(msg),
            EngineErrorCode::PayloadTooLarge => RouterError::PayloadTooLarge,
            EngineErrorCode::RateLimited => RouterError::RateLimited,
            EngineErrorCode::PermissionDenied => RouterError::PermissionDenied(msg),
            EngineErrorCode::Forbidden => RouterError::Forbidden(msg),
            EngineErrorCode::NotFound => RouterError::NotFound(msg),
            EngineErrorCode::DatabaseBusy => RouterError::DatabaseBusy(msg),
            EngineErrorCode::StorageCorruption => RouterError::StorageCorruption(msg),
            EngineErrorCode::ServiceUnavailable => RouterError::ServiceUnavailable,
            EngineErrorCode::Internal => RouterError::Internal(msg),
        }
    }
}

impl RouterError {
    pub fn status_code(&self) -> u16 {
        match self {
            RouterError::NotFound(_) => 404,
            RouterError::BadRequest(_) => 400,
            RouterError::InvalidAql(_) => 400,
            RouterError::UnknownField(_) => 400,
            RouterError::UnsupportedOperator(_) => 400,
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
