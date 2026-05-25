#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KnowledgeCellType {
    DocumentBlock,
    Fact,
    Entity,
    Relation,
    Memory,
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
        if let Some(source) = &self.metadata.source {
            lines.push(format!("source={}", sanitize_line_value(source)));
        }
        lines.push(String::new());
        let mut payload = lines.join("\n").into_bytes();
        payload.extend_from_slice(&self.body);
        payload
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
            source: None,
        }
    }
}

impl KnowledgeCellType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::DocumentBlock => "document_block",
            Self::Fact => "fact",
            Self::Entity => "entity",
            Self::Relation => "relation",
            Self::Memory => "memory",
            Self::Tool => "tool",
            Self::SourceRef => "source_ref",
            Self::Raw => "raw",
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
