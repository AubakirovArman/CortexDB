use super::metadata::KnowledgeCellMetadata;
use super::types::KnowledgeCellType;

#[path = "descriptor_default.rs"]
mod default_impl;
#[path = "descriptor_section.rs"]
mod section;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CellDescriptor {
    pub scope: String,
    pub status: String,
    pub cell_type: KnowledgeCellType,
    pub memory_type: Option<String>,
    pub ttl_seconds: Option<u64>,
    pub created_unix_seconds: Option<u64>,
    pub source_trust_q16: Option<u16>,
    pub source: Option<String>,
    pub citation: Option<String>,
    pub content_hash: Option<String>,
    pub source_id: Option<String>,
    pub source_url: Option<String>,
    pub document_id: Option<String>,
    pub page: Option<u32>,
    pub row: Option<u32>,
    pub cell_range: Option<String>,
    pub json_path: Option<String>,
    pub confidence_q16: Option<u16>,
    pub parent_id: Option<String>,
    pub valid_from: Option<String>,
    pub valid_to: Option<String>,
    pub session_id: Option<String>,
    pub session_kind: Option<String>,
}

impl CellDescriptor {
    pub fn from_metadata(metadata: &KnowledgeCellMetadata) -> Self {
        Self {
            scope: metadata.scope.clone(),
            status: metadata.status.clone(),
            cell_type: metadata.cell_type,
            memory_type: metadata.memory_type.clone(),
            ttl_seconds: metadata.ttl_seconds,
            created_unix_seconds: metadata.created_unix_seconds,
            source_trust_q16: metadata.source_trust_q16,
            source: metadata.source.clone(),
            citation: None,
            content_hash: None,
            source_id: None,
            source_url: None,
            document_id: None,
            page: None,
            row: None,
            cell_range: None,
            json_path: None,
            confidence_q16: None,
            parent_id: None,
            valid_from: None,
            valid_to: None,
            session_id: None,
            session_kind: None,
        }
    }

    pub fn from_payload_lossy(payload: &[u8]) -> Self {
        let text = String::from_utf8_lossy(payload);
        let mut descriptor = Self::default();
        for line in text.lines() {
            if line.trim().is_empty() {
                break;
            }
            let Some((key, value)) = line.split_once('=') else {
                break;
            };
            descriptor.apply_header(key.trim(), value.trim());
        }
        descriptor
    }

    pub fn from_metadata_section_lossy(metadata: &[u8]) -> Self {
        let text = String::from_utf8_lossy(metadata);
        let mut descriptor = Self::default();
        for (line_index, line) in text.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() {
                break;
            }
            if line_index == 0 && line == "cortexdb.cell_metadata.v1" {
                continue;
            }
            let Some((key, value)) = line.split_once('=') else {
                break;
            };
            descriptor.apply_header(key.trim(), value.trim());
        }
        descriptor
    }
}
