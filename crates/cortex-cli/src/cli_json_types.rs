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
pub struct CliAnnSearchReportResponse {
    pub path: String,
    pub fallback_reason: Option<String>,
    pub requested_limit: usize,
    pub allowed_candidates: usize,
    pub graph_nodes: usize,
    pub returned_candidates: usize,
    pub recall_q16: Option<u16>,
    pub min_recall_q16: Option<u16>,
}

#[derive(Serialize)]
pub struct CliAnnEvaluationResponse {
    pub available: bool,
    pub reason: Option<String>,
    pub ann_report: Option<CliAnnSearchReportResponse>,
    pub exact_top_k: Vec<u32>,
    pub ann_top_k: Vec<u32>,
    pub overlap_count: usize,
    pub recall_q16: u16,
}

#[derive(Serialize)]
pub struct ContextPackResponse {
    pub token_budget_tokens: u32,
    pub estimated_tokens: u32,
    pub truncated: bool,
    pub citations_required: bool,
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
    pub base_bm25: u32,
    pub source_trust_bonus: u32,
    pub redundancy_penalty: u32,
}

#[derive(Serialize)]
pub struct SourceRefResponse {
    pub source_id: String,
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
    pub citation: Option<String>,
    pub payload_text: String,
}

#[derive(Serialize)]
pub struct NumericConflictResponse {
    pub metric: String,
    pub left: String,
    pub right: String,
}
