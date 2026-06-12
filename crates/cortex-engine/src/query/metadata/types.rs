use cortex_aql::MemoryType;

use crate::source_trust::SourceTrustClass;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceRef {
    pub source_id: String,
    pub source_url: Option<String>,
    pub document_id: Option<String>,
    pub page: Option<u32>,
    pub row: Option<u32>,
    pub cell_range: Option<String>,
    pub json_path: Option<String>,
    pub confidence_q16: u16,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CellMetadata {
    pub scope: String,
    pub status: String,
    pub cell_type: String,
    pub memory_type: Option<MemoryType>,
    pub ttl_seconds: Option<u64>,
    pub created_unix_seconds: Option<u64>,
    pub source_trust_q16: Option<u16>,
    pub source_trust_class: Option<SourceTrustClass>,
    pub source: Option<String>,
    pub citation: Option<String>,
    pub title: Option<String>,
    pub content_hash: Option<String>,
    pub source_hash: Option<String>,
    pub document_id: Option<String>,
    pub chunk_id: Option<String>,
    pub parent_id: Option<String>,
    pub chunk_role: Option<String>,
    pub path: Option<String>,
    pub section: Option<String>,
    pub project: Option<String>,
    pub entity: Option<String>,
    pub sector: Option<String>,
    pub owner: Option<String>,
    pub status_tag: Option<String>,
    pub event_date: Option<String>,
    pub topic: Option<String>,
    pub as_of: Option<String>,
    pub valid_from: Option<String>,
    pub valid_to: Option<String>,
    pub supersedes: Option<String>,
    pub superseded_by: Option<String>,
    pub table_id: Option<String>,
    pub table_headers: Option<String>,
    pub row_label: Option<String>,
    pub body_text: String,
    pub terms: Vec<String>,
    pub source_ref: Option<SourceRef>,
}
