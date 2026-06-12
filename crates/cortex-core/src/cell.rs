#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KnowledgeCellType {
    DocumentBlock,
    Table,
    Fact,
    Entity,
    Relation,
    Memory,
    Feedback,
    Tool,
    SourceRef,
    Raw,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KnowledgeCellMetadata {
    pub scope: String,
    pub status: String,
    pub cell_type: KnowledgeCellType,
    pub memory_type: Option<String>,
    pub ttl_seconds: Option<u64>,
    pub created_unix_seconds: Option<u64>,
    pub source_trust_q16: Option<u16>,
    pub source: Option<String>,
}

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
    pub parent_id: Option<String>,
    pub valid_from: Option<String>,
    pub valid_to: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KnowledgeCell {
    pub metadata: KnowledgeCellMetadata,
    pub body: Vec<u8>,
}

impl KnowledgeCell {
    pub fn new(metadata: KnowledgeCellMetadata, body: impl Into<Vec<u8>>) -> Self {
        Self {
            metadata,
            body: body.into(),
        }
    }

    pub fn encode_payload(&self) -> Vec<u8> {
        let mut lines = vec![
            format!("scope={}", sanitize_line_value(&self.metadata.scope)),
            format!("status={}", sanitize_line_value(&self.metadata.status)),
            format!("type={}", self.metadata.cell_type.as_str()),
        ];
        if let Some(memory_type) = &self.metadata.memory_type {
            lines.push(format!("memory_type={}", sanitize_line_value(memory_type)));
        }
        if let Some(ttl_seconds) = self.metadata.ttl_seconds {
            lines.push(format!("ttl_seconds={ttl_seconds}"));
        }
        if let Some(created_unix_seconds) = self.metadata.created_unix_seconds {
            lines.push(format!("created_unix_seconds={created_unix_seconds}"));
        }
        if let Some(source_trust_q16) = self.metadata.source_trust_q16 {
            lines.push(format!("source_trust_q16={source_trust_q16}"));
        }
        if let Some(source) = &self.metadata.source {
            lines.push(format!("source={}", sanitize_line_value(source)));
        }
        lines.push(String::new());
        let mut payload = lines.join("\n").into_bytes();
        payload.extend_from_slice(&self.body);
        payload
    }
}

impl KnowledgeCellMetadata {
    pub fn encode_wal_section(&self) -> Vec<u8> {
        let mut lines = vec![
            "cortexdb.cell_metadata.v1".to_owned(),
            format!("scope={}", sanitize_line_value(&self.scope)),
            format!("status={}", sanitize_line_value(&self.status)),
            format!("type={}", self.cell_type.as_str()),
        ];
        if let Some(memory_type) = &self.memory_type {
            lines.push(format!("memory_type={}", sanitize_line_value(memory_type)));
        }
        if let Some(ttl_seconds) = self.ttl_seconds {
            lines.push(format!("ttl_seconds={ttl_seconds}"));
        }
        if let Some(created_unix_seconds) = self.created_unix_seconds {
            lines.push(format!("created_unix_seconds={created_unix_seconds}"));
        }
        if let Some(source_trust_q16) = self.source_trust_q16 {
            lines.push(format!("source_trust_q16={source_trust_q16}"));
        }
        if let Some(source) = &self.source {
            lines.push(format!("source={}", sanitize_line_value(source)));
        }
        lines.join("\n").into_bytes()
    }
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

    fn apply_header(&mut self, key: &str, value: &str) {
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
            "parent_id" | "parent_chunk_id" => self.parent_id = non_empty(value),
            "valid_from" => self.valid_from = non_empty(value),
            "valid_to" => self.valid_to = non_empty(value),
            _ => {}
        }
    }
}

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
            parent_id: None,
            valid_from: None,
            valid_to: None,
        }
    }
}

impl Default for KnowledgeCellMetadata {
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
        }
    }
}

impl KnowledgeCellType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::DocumentBlock => "document_block",
            Self::Table => "table",
            Self::Fact => "fact",
            Self::Entity => "entity",
            Self::Relation => "relation",
            Self::Memory => "memory",
            Self::Feedback => "feedback",
            Self::Tool => "tool",
            Self::SourceRef => "source_ref",
            Self::Raw => "raw",
        }
    }
}

impl std::str::FromStr for KnowledgeCellType {
    type Err = ();

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "document_block" => Ok(Self::DocumentBlock),
            "table" => Ok(Self::Table),
            "fact" => Ok(Self::Fact),
            "entity" => Ok(Self::Entity),
            "relation" => Ok(Self::Relation),
            "memory" => Ok(Self::Memory),
            "feedback" => Ok(Self::Feedback),
            "tool" => Ok(Self::Tool),
            "source_ref" => Ok(Self::SourceRef),
            "raw" => Ok(Self::Raw),
            _ => Err(()),
        }
    }
}

fn sanitize_line_value(value: &str) -> String {
    value
        .chars()
        .map(|character| match character {
            '\n' | '\r' => ' ',
            other => other,
        })
        .collect()
}

fn non_empty(value: &str) -> Option<String> {
    let trimmed = value.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_owned())
}

#[cfg(test)]
mod tests {
    use super::{CellDescriptor, KnowledgeCell, KnowledgeCellMetadata, KnowledgeCellType};

    #[test]
    fn descriptor_decodes_stable_payload_headers() {
        let payload = b"scope=project:investments\nstatus=verified\ntype=fact\nmemory_type=decision\nttl_seconds=3600\ncreated_unix_seconds=1710000000\nsource_trust_q16=60000\nsource=annual-report\ncitation=p12\ncontent_hash=abc\nparent_chunk_id=doc-1\nvalid_from=2024-01-01\nvalid_to=2024-12-31\n\nbody";
        let descriptor = CellDescriptor::from_payload_lossy(payload);

        assert_eq!(descriptor.scope, "project:investments");
        assert_eq!(descriptor.status, "verified");
        assert_eq!(descriptor.cell_type, KnowledgeCellType::Fact);
        assert_eq!(descriptor.memory_type.as_deref(), Some("decision"));
        assert_eq!(descriptor.ttl_seconds, Some(3600));
        assert_eq!(descriptor.created_unix_seconds, Some(1_710_000_000));
        assert_eq!(descriptor.source_trust_q16, Some(60_000));
        assert_eq!(descriptor.source.as_deref(), Some("annual-report"));
        assert_eq!(descriptor.citation.as_deref(), Some("p12"));
        assert_eq!(descriptor.content_hash.as_deref(), Some("abc"));
        assert_eq!(descriptor.parent_id.as_deref(), Some("doc-1"));
        assert_eq!(descriptor.valid_from.as_deref(), Some("2024-01-01"));
        assert_eq!(descriptor.valid_to.as_deref(), Some("2024-12-31"));
    }

    #[test]
    fn descriptor_matches_encoded_knowledge_cell_metadata() {
        let cell = KnowledgeCell::new(
            KnowledgeCellMetadata {
                scope: "agent:7".to_owned(),
                status: "ready".to_owned(),
                cell_type: KnowledgeCellType::Memory,
                memory_type: Some("preference".to_owned()),
                ttl_seconds: Some(30),
                created_unix_seconds: Some(42),
                source_trust_q16: Some(50_000),
                source: Some("user".to_owned()),
            },
            "remember this",
        );

        let descriptor = CellDescriptor::from_payload_lossy(&cell.encode_payload());
        assert_eq!(descriptor, CellDescriptor::from_metadata(&cell.metadata));
    }
}
