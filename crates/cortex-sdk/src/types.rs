use serde::Deserialize;

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
    pub requested_limit: usize,
    pub allowed_candidates: usize,
    pub graph_nodes: usize,
    pub returned_candidates: usize,
    pub recall_q16: Option<u16>,
    pub min_recall_q16: Option<u16>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct AnnEvaluationResponse {
    pub available: bool,
    pub reason: Option<String>,
    pub ann_report: Option<AnnSearchReport>,
    pub exact_top_k: Vec<u32>,
    pub ann_top_k: Vec<u32>,
    pub overlap_count: usize,
    pub recall_q16: u16,
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
    pub ann_report: Option<AnnSearchReport>,
    pub results: Vec<SearchResult>,
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
pub struct AqlResponse {
    pub cells: Vec<AqlCellResponse>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct ExplainResponse {
    pub score: u32,
    pub matched_terms: Vec<String>,
    pub why_selected: String,
    pub base_bm25: u32,
    pub source_trust_bonus: u32,
    pub redundancy_penalty: u32,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct SourceRefResponse {
    pub source_id: String,
    pub document_id: Option<String>,
    pub page: Option<u32>,
    pub cell_range: Option<String>,
    pub json_path: Option<String>,
    pub confidence_q16: u16,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct ContextPackCellResponse {
    pub cell_id: u64,
    pub estimated_tokens: u32,
    pub citation: Option<String>,
    pub payload_text: String,
    pub explain: Option<ExplainResponse>,
    pub source_ref: Option<SourceRefResponse>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct ContextPackAnomalyResponse {
    pub cell_id: Option<u64>,
    pub code: String,
    pub message: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct ContextPackResponse {
    pub token_budget_tokens: u32,
    pub estimated_tokens: u32,
    pub truncated: bool,
    pub citations_required: bool,
    pub cells: Vec<ContextPackCellResponse>,
    pub anomalies: Vec<ContextPackAnomalyResponse>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct EvidenceResponse {
    pub cell_id: u64,
    pub matched_terms: u32,
    pub source_trust_q16: u16,
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
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct RememberResponse {
    pub seq: u64,
    pub cell_id: u64,
    pub ttl_seconds: Option<u64>,
}
