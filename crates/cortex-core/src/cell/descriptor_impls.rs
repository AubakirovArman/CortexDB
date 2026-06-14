use super::descriptor::CellDescriptor;
use super::types::KnowledgeCellType;
use super::wire::{
    decode_non_empty_string, decode_optional_string, non_empty, read_fixed_u16, read_fixed_u64,
};

impl CellDescriptor {
    pub fn overlay_metadata_descriptor(&mut self, metadata: &Self) {
        self.scope = metadata.scope.clone();
        self.status = metadata.status.clone();
        self.cell_type = metadata.cell_type;
        if metadata.memory_type.is_some() {
            self.memory_type = metadata.memory_type.clone();
        }
        if metadata.ttl_seconds.is_some() {
            self.ttl_seconds = metadata.ttl_seconds;
        }
        if metadata.created_unix_seconds.is_some() {
            self.created_unix_seconds = metadata.created_unix_seconds;
        }
        self.overlay_optional_provenance(metadata);
    }

    fn overlay_optional_provenance(&mut self, metadata: &Self) {
        if metadata.source_trust_q16.is_some() {
            self.source_trust_q16 = metadata.source_trust_q16;
        }
        if metadata.source.is_some() {
            self.source = metadata.source.clone();
        }
        if metadata.citation.is_some() {
            self.citation = metadata.citation.clone();
        }
        if metadata.content_hash.is_some() {
            self.content_hash = metadata.content_hash.clone();
        }
        if metadata.source_id.is_some() {
            self.source_id = metadata.source_id.clone();
        }
        if metadata.source_url.is_some() {
            self.source_url = metadata.source_url.clone();
        }
        if metadata.document_id.is_some() {
            self.document_id = metadata.document_id.clone();
        }
        if metadata.page.is_some() {
            self.page = metadata.page;
        }
        if metadata.row.is_some() {
            self.row = metadata.row;
        }
        if metadata.cell_range.is_some() {
            self.cell_range = metadata.cell_range.clone();
        }
        if metadata.json_path.is_some() {
            self.json_path = metadata.json_path.clone();
        }
        if metadata.confidence_q16.is_some() {
            self.confidence_q16 = metadata.confidence_q16;
        }
        if metadata.session_id.is_some() {
            self.session_id = metadata.session_id.clone();
        }
        if metadata.session_kind.is_some() {
            self.session_kind = metadata.session_kind.clone();
        }
    }

    pub(crate) fn apply_header(&mut self, key: &str, value: &str) {
        match key {
            "scope" if !value.is_empty() => self.scope = value.to_owned(),
            "status" if !value.is_empty() => self.status = value.to_owned(),
            "type" => {
                if let Ok(cell_type) = value.parse() {
                    self.cell_type = cell_type;
                }
            }
            "memory_type" => self.memory_type = non_empty(value),
            "ttl_seconds" => self.ttl_seconds = value.parse().ok(),
            "created_unix_seconds" => self.created_unix_seconds = value.parse().ok(),
            "source_trust_q16" => self.source_trust_q16 = value.parse().ok(),
            "source" => self.source = non_empty(value),
            "citation" => self.citation = non_empty(value),
            "content_hash" => self.content_hash = non_empty(value),
            "source_id" => self.source_id = non_empty(value),
            "source_url" | "url" => self.source_url = non_empty(value),
            "document_id" | "doc_id" => self.document_id = non_empty(value),
            "page" => self.page = value.parse().ok(),
            "row" | "row_number" => self.row = value.parse().ok(),
            "cell_range" | "chunk_id" => self.cell_range = non_empty(value),
            "json_path" => self.json_path = non_empty(value),
            "confidence_q16" => self.confidence_q16 = value.parse().ok(),
            "parent_id" | "parent_chunk_id" => self.parent_id = non_empty(value),
            "valid_from" => self.valid_from = non_empty(value),
            "valid_to" => self.valid_to = non_empty(value),
            "session_id" => self.session_id = non_empty(value),
            "session_kind" => self.session_kind = non_empty(value),
            _ => {}
        }
    }

    pub(crate) fn apply_binary_field(&mut self, tag: u8, value: &[u8]) -> Option<()> {
        match tag {
            1 => self.scope = decode_non_empty_string(value)?,
            2 => self.status = decode_non_empty_string(value)?,
            3 if value.len() == 1 => self.cell_type = KnowledgeCellType::from_u8(value[0])?,
            4 => self.memory_type = decode_optional_string(value)?,
            5 if value.len() == 8 => self.ttl_seconds = Some(read_fixed_u64(value)?),
            6 if value.len() == 8 => self.created_unix_seconds = Some(read_fixed_u64(value)?),
            7 if value.len() == 2 => self.source_trust_q16 = Some(read_fixed_u16(value)?),
            8 => self.source = decode_optional_string(value)?,
            9 => self.citation = decode_optional_string(value)?,
            10 => self.content_hash = decode_optional_string(value)?,
            11 => self.parent_id = decode_optional_string(value)?,
            12 => self.valid_from = decode_optional_string(value)?,
            13 => self.valid_to = decode_optional_string(value)?,
            14 => self.source_id = decode_optional_string(value)?,
            15 => self.source_url = decode_optional_string(value)?,
            16 => self.document_id = decode_optional_string(value)?,
            17 if value.len() == 8 => self.page = read_fixed_u64(value)?.try_into().ok(),
            18 if value.len() == 8 => self.row = read_fixed_u64(value)?.try_into().ok(),
            19 => self.cell_range = decode_optional_string(value)?,
            20 => self.json_path = decode_optional_string(value)?,
            21 if value.len() == 2 => self.confidence_q16 = Some(read_fixed_u16(value)?),
            22 => self.session_id = decode_optional_string(value)?,
            23 => self.session_kind = decode_optional_string(value)?,
            _ => {}
        }
        Some(())
    }
}
