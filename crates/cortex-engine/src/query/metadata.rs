use std::collections::BTreeMap;

use cortex_aql::{BitmapHandle, CellTypeId, MemoryType, ScopeId, StatusId};

use crate::search::tokenize;

const SCOPE_NS: u64 = 0x1000_0000_0000_0000;
const STATUS_NS: u64 = 0x2000_0000_0000_0000;
const TYPE_NS: u64 = 0x3000_0000_0000_0000;
const MEMORY_NS: u64 = 0x4000_0000_0000_0000;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceRef {
    pub source_id: String,
    pub document_id: Option<String>,
    pub page: Option<u32>,
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
    pub source: Option<String>,
    pub citation: Option<String>,
    pub title: Option<String>,
    pub body_text: String,
    pub terms: Vec<String>,
    pub source_ref: Option<SourceRef>,
}

impl CellMetadata {
    pub fn from_payload(payload: &[u8]) -> Self {
        let text = String::from_utf8_lossy(payload);
        let mut scope = "default".to_owned();
        let mut status = "ready".to_owned();
        let mut cell_type = "raw".to_owned();
        let mut memory_type = None;
        let mut ttl_seconds = None;
        let mut created_unix_seconds = None;
        let mut source_trust_q16 = None;
        let mut source = None;
        let mut citation = None;
        let mut title = None;
        let mut body_lines = Vec::new();
        let mut in_header = true;

        let mut source_id_val = None;
        let mut document_id = None;
        let mut page = None;
        let mut cell_range = None;
        let mut json_path = None;
        let mut confidence_q16 = None;

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
                } else if let Some(value) = line.strip_prefix("title=") {
                    title = non_empty(value);
                    continue;
                } else if let Some(value) = line.strip_prefix("source_id=") {
                    source_id_val = non_empty(value);
                    continue;
                } else if let Some(value) = line.strip_prefix("document_id=") {
                    document_id = non_empty(value);
                    continue;
                } else if let Some(value) = line.strip_prefix("page=") {
                    page = value.trim().parse().ok();
                    continue;
                } else if let Some(value) = line.strip_prefix("cell_range=") {
                    cell_range = non_empty(value);
                    continue;
                } else if let Some(value) = line.strip_prefix("json_path=") {
                    json_path = non_empty(value);
                    continue;
                } else if let Some(value) = line.strip_prefix("confidence_q16=") {
                    confidence_q16 = value.trim().parse().ok();
                    continue;
                }
                in_header = false;
            }
            body_lines.push(line);
        }
        let body_text = body_lines.join("\n");
        let terms = tokenize(&body_text);

        let final_source_id = source_id_val
            .or_else(|| source.clone())
            .or_else(|| citation.clone());
        let source_ref = final_source_id.map(|id| SourceRef {
            source_id: id,
            document_id,
            page,
            cell_range,
            json_path,
            confidence_q16: confidence_q16.or(source_trust_q16).unwrap_or(32768),
        });

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
            title,
            body_text,
            terms,
            source_ref,
        }
    }

    pub fn citation(&self) -> Option<&str> {
        self.citation.as_deref().or(self.source.as_deref())
    }

    pub fn weighted_lexical_terms(&self) -> BTreeMap<String, u32> {
        let mut terms = BTreeMap::new();
        add_weighted_terms(&mut terms, &self.body_text, 1);
        if let Some(title) = &self.title {
            add_weighted_terms(&mut terms, title, 6);
        }
        terms
    }
}

fn non_empty(value: &str) -> Option<String> {
    let value = value.trim();
    (!value.is_empty()).then(|| value.to_owned())
}

fn add_weighted_terms(terms: &mut BTreeMap<String, u32>, text: &str, weight: u32) {
    for term in tokenize(text) {
        *terms.entry(term).or_default() += weight;
    }
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
