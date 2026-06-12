use super::types::KnowledgeCellType;
use super::wire::sanitize_line_value;

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
