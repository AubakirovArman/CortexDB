use cortex_aql::{BitmapHandle, CellTypeId, MemoryType, ScopeId, StatusId};

use crate::search::tokenize;

const SCOPE_NS: u64 = 0x1000_0000_0000_0000;
const STATUS_NS: u64 = 0x2000_0000_0000_0000;
const TYPE_NS: u64 = 0x3000_0000_0000_0000;
const MEMORY_NS: u64 = 0x4000_0000_0000_0000;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CellMetadata {
    pub scope: String,
    pub status: String,
    pub cell_type: String,
    pub memory_type: Option<MemoryType>,
    pub ttl_seconds: Option<u64>,
    pub created_unix_seconds: Option<u64>,
    pub source_trust_q16: Option<u16>,
    pub source: Option<String>,
    pub citation: Option<String>,
    pub body_text: String,
    pub terms: Vec<String>,
}

impl CellMetadata {
    pub fn from_payload(payload: &[u8]) -> Self {
        let text = String::from_utf8_lossy(payload);
        let mut scope = "default".to_owned();
        let mut status = "ready".to_owned();
        let mut cell_type = "cell".to_owned();
        let mut memory_type = None;
        let mut ttl_seconds = None;
        let mut created_unix_seconds = None;
        let mut source_trust_q16 = None;
        let mut source = None;
        let mut citation = None;
        let mut body_lines = Vec::new();
        let mut in_header = true;
        for line in text.lines() {
            if in_header {
                if line.trim().is_empty() {
                    in_header = false;
                    continue;
                }
                if let Some(value) = line.strip_prefix("scope=") {
                    scope = value.trim().to_owned();
                    continue;
                } else if let Some(value) = line.strip_prefix("status=") {
                    status = value.trim().to_owned();
                    continue;
                } else if let Some(value) = line.strip_prefix("type=") {
                    cell_type = value.trim().to_owned();
                    continue;
                } else if let Some(value) = line.strip_prefix("memory_type=") {
                    memory_type = value.trim().parse().ok();
                    continue;
                } else if let Some(value) = line.strip_prefix("ttl_seconds=") {
                    ttl_seconds = value.trim().parse().ok();
                    continue;
                } else if let Some(value) = line.strip_prefix("created_unix_seconds=") {
                    created_unix_seconds = value.trim().parse().ok();
                    continue;
                } else if let Some(value) = line.strip_prefix("source_trust_q16=") {
                    source_trust_q16 = value.trim().parse().ok();
                    continue;
                } else if let Some(value) = line.strip_prefix("source=") {
                    source = non_empty(value);
                    continue;
                } else if let Some(value) = line.strip_prefix("citation=") {
                    citation = non_empty(value);
                    continue;
                }
                in_header = false;
            }
            body_lines.push(line);
        }
        let body_text = body_lines.join("\n");
        let terms = tokenize(&body_text);
        Self {
            scope,
            status,
            cell_type,
            memory_type,
            ttl_seconds,
            created_unix_seconds,
            source_trust_q16,
            source,
            citation,
            body_text,
            terms,
        }
    }

    pub fn citation(&self) -> Option<&str> {
        self.citation.as_deref().or(self.source.as_deref())
    }
}

fn non_empty(value: &str) -> Option<String> {
    let value = value.trim();
    (!value.is_empty()).then(|| value.to_owned())
}

pub fn scope_id(name: &str) -> ScopeId {
    ScopeId(stable_hash(name))
}

pub(crate) fn status_id(name: &str) -> StatusId {
    StatusId(stable_hash(name))
}

pub(crate) fn cell_type_id(name: &str) -> CellTypeId {
    CellTypeId(stable_hash(name))
}

pub(crate) fn scope_handle(scope: ScopeId) -> BitmapHandle {
    BitmapHandle(SCOPE_NS | scope.0)
}

pub(crate) fn status_handle(status: StatusId) -> BitmapHandle {
    BitmapHandle(STATUS_NS | status.0)
}

pub(crate) fn cell_type_handle(cell_type: CellTypeId) -> BitmapHandle {
    BitmapHandle(TYPE_NS | cell_type.0)
}

pub(crate) fn memory_type_handle(memory_type: MemoryType) -> BitmapHandle {
    BitmapHandle(MEMORY_NS | memory_type as u64)
}

fn stable_hash(value: &str) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for byte in value.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash & 0x0fff_ffff_ffff_ffff
}
