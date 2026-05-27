use serde::Serialize;

#[derive(Serialize, Debug, Clone)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
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
    pub cell_id: u64,
    pub code: String,
    pub message: String,
}

#[derive(Serialize, Debug, Clone)]
pub struct ContextPackResponse {
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
    pub requested_limit: usize,
    pub allowed_candidates: usize,
    pub graph_nodes: usize,
    pub returned_candidates: usize,
    pub recall_q16: Option<u16>,
    pub min_recall_q16: Option<u16>,
}

#[derive(Serialize, Debug, Clone)]
pub struct SearchResponse {
    pub search_mode: String,
    pub ann_report: Option<AnnSearchReportResponse>,
    pub results: Vec<SearchResultResponse>,
}

#[derive(Serialize, Debug, Clone)]
pub struct AnnEvaluationResponse {
    pub available: bool,
    pub reason: Option<String>,
    pub ann_report: Option<AnnSearchReportResponse>,
    pub exact_top_k: Vec<u32>,
    pub ann_top_k: Vec<u32>,
    pub overlap_count: usize,
    pub recall_q16: u16,
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
}

#[derive(Serialize, Debug, Clone)]
pub struct ErrorResponse {
    pub error: String,
    pub message: String,
}

/// Typed router error taxonomy for consistent HTTP status mapping.
#[derive(Serialize, Debug, Clone)]
pub struct AnnMetricsResponse {
    pub graph_nodes: usize,
    pub total_edges: usize,
    pub persisted_segments: usize,
    pub has_checkpoint: bool,
    pub has_uncheckpointed_changes: bool,
}

pub enum RouterError {
    NotFound(String),
    BadRequest(String),
    Unauthorized,
    PayloadTooLarge,
    ServiceUnavailable,
    Internal(String),
}

impl std::fmt::Display for RouterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RouterError::NotFound(msg) => write!(f, "{msg}"),
            RouterError::BadRequest(msg) => write!(f, "{msg}"),
            RouterError::Unauthorized => write!(f, "missing or invalid authorization"),
            RouterError::PayloadTooLarge => write!(f, "request body exceeds 2MB limit"),
            RouterError::ServiceUnavailable => write!(f, "database actor busy"),
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

impl RouterError {
    pub fn status_code(&self) -> u16 {
        match self {
            RouterError::NotFound(_) => 404,
            RouterError::BadRequest(_) => 400,
            RouterError::Unauthorized => 401,
            RouterError::PayloadTooLarge => 413,
            RouterError::ServiceUnavailable => 503,
            RouterError::Internal(_) => 500,
        }
    }
}
