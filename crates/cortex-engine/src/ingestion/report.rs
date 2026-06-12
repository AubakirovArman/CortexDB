use serde::{Deserialize, Serialize};

use crate::database::Database;
use crate::ingestion::IngestedCell;
use crate::query::CellMetadata;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct IngestionValidationReport {
    pub cells_seen: usize,
    pub processed_records: usize,
    pub skipped_records: usize,
    pub invalid_metadata_records: usize,
    pub warnings: Vec<IngestionValidationIssue>,
    pub skipped_items: Vec<IngestionSkippedItem>,
    pub source_refs: Vec<IngestionSourceRefReport>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct IngestionValidationIssue {
    pub code: String,
    pub message: String,
    pub cell_id: Option<u64>,
    pub chunk_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct IngestionSkippedItem {
    pub reason: String,
    pub input_ref: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct IngestionSourceRefReport {
    pub cell_id: u64,
    pub chunk_id: Option<String>,
    pub has_source_ref: bool,
    pub source_id: Option<String>,
    pub source_url: Option<String>,
    pub document_id: Option<String>,
    pub page: Option<u32>,
    pub row: Option<u32>,
    pub cell_range: Option<String>,
    pub json_path: Option<String>,
    pub confidence_q16: Option<u16>,
}

impl Database {
    pub fn ingestion_validation_report(&self, cells: &[IngestedCell]) -> IngestionValidationReport {
        let mut report = IngestionValidationReport {
            cells_seen: cells.len(),
            processed_records: cells.len(),
            ..IngestionValidationReport::default()
        };
        for cell in cells {
            let Some((payload, descriptor)) = self.get_latest_cell_with_descriptor(cell.cell_id)
            else {
                report.warn(
                    "missing_payload",
                    "ingested cell is not visible after write",
                    Some(cell.cell_id.0),
                    cell.chunk_id.clone(),
                );
                continue;
            };
            if let Err(error) = CellMetadata::decode_payload(&payload) {
                report.record_invalid_metadata(cell.cell_id.0, cell.chunk_id.clone(), &error);
            }
            let metadata = CellMetadata::from_payload_with_descriptor(&payload, &descriptor);
            let source_ref = metadata.source_ref;
            if let (Some(expected), Some(actual)) = (
                cell.chunk_id.as_deref(),
                source_ref.as_ref().and_then(|s| s.cell_range.as_deref()),
            ) {
                if expected != actual {
                    report.warn(
                        "chunk_id_mismatch",
                        "ingested chunk id does not match SourceRef cell range",
                        Some(cell.cell_id.0),
                        cell.chunk_id.clone(),
                    );
                }
            }
            if source_ref.is_none() {
                report.warn(
                    "missing_source_ref",
                    "ingested cell has no structured SourceRef metadata",
                    Some(cell.cell_id.0),
                    cell.chunk_id.clone(),
                );
            }
            report.source_refs.push(IngestionSourceRefReport {
                cell_id: cell.cell_id.0,
                chunk_id: cell.chunk_id.clone(),
                has_source_ref: source_ref.is_some(),
                source_id: source_ref.as_ref().map(|value| value.source_id.clone()),
                source_url: source_ref
                    .as_ref()
                    .and_then(|value| value.source_url.clone()),
                document_id: source_ref
                    .as_ref()
                    .and_then(|value| value.document_id.clone()),
                page: source_ref.as_ref().and_then(|value| value.page),
                row: source_ref.as_ref().and_then(|value| value.row),
                cell_range: source_ref
                    .as_ref()
                    .and_then(|value| value.cell_range.clone()),
                json_path: source_ref
                    .as_ref()
                    .and_then(|value| value.json_path.clone()),
                confidence_q16: source_ref.as_ref().map(|value| value.confidence_q16),
            });
        }
        report
    }
}

impl IngestionValidationReport {
    pub fn record_skipped(&mut self, reason: &str, input_ref: Option<String>) {
        self.skipped_records += 1;
        self.skipped_items.push(IngestionSkippedItem {
            reason: reason.to_owned(),
            input_ref,
        });
    }

    fn record_invalid_metadata(
        &mut self,
        cell_id: u64,
        chunk_id: Option<String>,
        error: &dyn core::fmt::Display,
    ) {
        self.invalid_metadata_records += 1;
        self.warn(
            "invalid_metadata",
            &format!("ingested cell metadata is invalid: {error}"),
            Some(cell_id),
            chunk_id,
        );
    }

    fn warn(&mut self, code: &str, message: &str, cell_id: Option<u64>, chunk_id: Option<String>) {
        self.warnings.push(IngestionValidationIssue {
            code: code.to_owned(),
            message: message.to_owned(),
            cell_id,
            chunk_id,
        });
    }
}
