use serde::Deserialize;

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
    #[serde(default)]
    pub source_url: Option<String>,
    pub document_id: Option<String>,
    #[serde(default)]
    pub page: Option<u32>,
    #[serde(default)]
    pub row: Option<u32>,
    #[serde(default)]
    pub cell_range: Option<String>,
    #[serde(default)]
    pub json_path: Option<String>,
    pub confidence_q16: Option<u16>,
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

fn default_max_retries() -> u32 {
    3
}
