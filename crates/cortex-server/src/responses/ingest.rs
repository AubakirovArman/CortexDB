use serde::Serialize;

use cortex_engine::IngestionValidationReport;

#[derive(Serialize, Debug, Clone)]
pub struct IngestResponse {
    pub rows_ingested: usize,
    pub chunks_ingested: usize,
    pub facts_ingested: usize,
    pub first_cell_id: Option<u64>,
    pub job_id: Option<u64>,
    pub validation_report: IngestionValidationReport,
}
