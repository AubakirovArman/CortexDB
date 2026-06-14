use super::super::types::KnowledgeCellType;
use super::CellDescriptor;

impl Default for CellDescriptor {
    fn default() -> Self {
        Self {
            scope: "default".to_owned(),
            status: "ready".to_owned(),
            cell_type: KnowledgeCellType::Raw,
            memory_type: None,
            ttl_seconds: None,
            created_unix_seconds: None,
            source_trust_q16: None,
            source: None,
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
}
