use super::metadata::KnowledgeCellMetadata;
use super::types::KnowledgeCellType;
use super::wire::{
    push_optional_string_field, push_optional_u16_field, push_optional_u64_field,
    push_string_field, push_u8_field, read_u32,
};

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
}

impl CellDescriptor {
    const SECTION_MAGIC_V1: [u8; 8] = *b"ACDESC1\0";

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

    pub fn encode_section_v1(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&Self::SECTION_MAGIC_V1);
        push_string_field(&mut out, 1, &self.scope);
        push_string_field(&mut out, 2, &self.status);
        push_u8_field(&mut out, 3, self.cell_type.to_u8());
        push_optional_string_field(&mut out, 4, self.memory_type.as_deref());
        push_optional_u64_field(&mut out, 5, self.ttl_seconds);
        push_optional_u64_field(&mut out, 6, self.created_unix_seconds);
        push_optional_u16_field(&mut out, 7, self.source_trust_q16);
        push_optional_string_field(&mut out, 8, self.source.as_deref());
        push_optional_string_field(&mut out, 9, self.citation.as_deref());
        push_optional_string_field(&mut out, 10, self.content_hash.as_deref());
        push_optional_string_field(&mut out, 11, self.parent_id.as_deref());
        push_optional_string_field(&mut out, 12, self.valid_from.as_deref());
        push_optional_string_field(&mut out, 13, self.valid_to.as_deref());
        push_optional_string_field(&mut out, 14, self.source_id.as_deref());
        push_optional_string_field(&mut out, 15, self.source_url.as_deref());
        push_optional_string_field(&mut out, 16, self.document_id.as_deref());
        push_optional_u64_field(&mut out, 17, self.page.map(u64::from));
        push_optional_u64_field(&mut out, 18, self.row.map(u64::from));
        push_optional_string_field(&mut out, 19, self.cell_range.as_deref());
        push_optional_string_field(&mut out, 20, self.json_path.as_deref());
        push_optional_u16_field(&mut out, 21, self.confidence_q16);
        out
    }

    pub fn decode_section_v1(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < Self::SECTION_MAGIC_V1.len()
            || bytes[..Self::SECTION_MAGIC_V1.len()] != Self::SECTION_MAGIC_V1
        {
            return None;
        }

        let mut cursor = Self::SECTION_MAGIC_V1.len();
        let mut descriptor = Self::default();
        while cursor < bytes.len() {
            let tag = *bytes.get(cursor)?;
            cursor += 1;
            let len = read_u32(bytes, &mut cursor)? as usize;
            let end = cursor.checked_add(len)?;
            let value = bytes.get(cursor..end)?;
            cursor = end;
            descriptor.apply_binary_field(tag, value)?;
        }
        Some(descriptor)
    }
}
