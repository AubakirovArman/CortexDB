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

    fn apply_binary_field(&mut self, tag: u8, value: &[u8]) -> Option<()> {
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
            _ => {}
        }
        Some(())
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

    fn to_u8(self) -> u8 {
        match self {
            Self::DocumentBlock => 1,
            Self::Table => 2,
            Self::Fact => 3,
            Self::Entity => 4,
            Self::Relation => 5,
            Self::Memory => 6,
            Self::Feedback => 7,
            Self::Tool => 8,
            Self::SourceRef => 9,
            Self::Raw => 10,
        }
    }

    fn from_u8(value: u8) -> Option<Self> {
        match value {
            1 => Some(Self::DocumentBlock),
            2 => Some(Self::Table),
            3 => Some(Self::Fact),
            4 => Some(Self::Entity),
            5 => Some(Self::Relation),
            6 => Some(Self::Memory),
            7 => Some(Self::Feedback),
            8 => Some(Self::Tool),
            9 => Some(Self::SourceRef),
            10 => Some(Self::Raw),
            _ => None,
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

fn push_string_field(out: &mut Vec<u8>, tag: u8, value: &str) {
    push_bytes_field(out, tag, value.as_bytes());
}

fn push_optional_string_field(out: &mut Vec<u8>, tag: u8, value: Option<&str>) {
    if let Some(value) = value {
        push_string_field(out, tag, value);
    }
}

fn push_u8_field(out: &mut Vec<u8>, tag: u8, value: u8) {
    push_bytes_field(out, tag, &[value]);
}

fn push_optional_u16_field(out: &mut Vec<u8>, tag: u8, value: Option<u16>) {
    if let Some(value) = value {
        push_bytes_field(out, tag, &value.to_le_bytes());
    }
}

fn push_optional_u64_field(out: &mut Vec<u8>, tag: u8, value: Option<u64>) {
    if let Some(value) = value {
        push_bytes_field(out, tag, &value.to_le_bytes());
    }
}

fn push_bytes_field(out: &mut Vec<u8>, tag: u8, value: &[u8]) {
    let Ok(len) = u32::try_from(value.len()) else {
        return;
    };
    out.push(tag);
    out.extend_from_slice(&len.to_le_bytes());
    out.extend_from_slice(value);
}

fn read_u32(bytes: &[u8], cursor: &mut usize) -> Option<u32> {
    let end = cursor.checked_add(4)?;
    let raw: [u8; 4] = bytes.get(*cursor..end)?.try_into().ok()?;
    *cursor = end;
    Some(u32::from_le_bytes(raw))
}

fn read_fixed_u16(bytes: &[u8]) -> Option<u16> {
    let raw: [u8; 2] = bytes.try_into().ok()?;
    Some(u16::from_le_bytes(raw))
}

fn read_fixed_u64(bytes: &[u8]) -> Option<u64> {
    let raw: [u8; 8] = bytes.try_into().ok()?;
    Some(u64::from_le_bytes(raw))
}

fn decode_non_empty_string(bytes: &[u8]) -> Option<String> {
    let value = String::from_utf8(bytes.to_vec()).ok()?;
    (!value.is_empty()).then_some(value)
}

fn decode_optional_string(bytes: &[u8]) -> Option<Option<String>> {
    Some(non_empty(&String::from_utf8(bytes.to_vec()).ok()?))
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

    #[test]
    fn descriptor_decodes_cell_metadata_section() {
        let metadata = KnowledgeCellMetadata {
            scope: "tenant:private".to_owned(),
            status: "ready".to_owned(),
            cell_type: KnowledgeCellType::Fact,
            memory_type: Some("decision".to_owned()),
            ttl_seconds: Some(600),
            created_unix_seconds: Some(1_710_000_001),
            source_trust_q16: Some(61_000),
            source: Some("source-b".to_owned()),
        };

        let descriptor =
            CellDescriptor::from_metadata_section_lossy(&metadata.encode_wal_section());

        assert_eq!(descriptor, CellDescriptor::from_metadata(&metadata));
    }

    #[test]
    fn descriptor_binary_section_roundtrips_and_skips_unknown_fields() {
        let descriptor = CellDescriptor {
            scope: "project:beta".to_owned(),
            status: "verified".to_owned(),
            cell_type: KnowledgeCellType::Fact,
            memory_type: Some("decision".to_owned()),
            ttl_seconds: Some(3600),
            created_unix_seconds: Some(1_710_000_000),
            source_trust_q16: Some(60_000),
            source: Some("source-a".to_owned()),
            citation: Some("p7".to_owned()),
            content_hash: Some("hash".to_owned()),
            parent_id: Some("parent".to_owned()),
            valid_from: Some("2024-01-01".to_owned()),
            valid_to: Some("2024-12-31".to_owned()),
        };
        let mut encoded = descriptor.encode_section_v1();
        encoded.push(250);
        encoded.extend_from_slice(&3u32.to_le_bytes());
        encoded.extend_from_slice(b"new");

        assert_eq!(
            CellDescriptor::decode_section_v1(&encoded),
            Some(descriptor)
        );
    }
}
