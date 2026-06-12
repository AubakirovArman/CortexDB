use std::collections::BTreeMap;

use cortex_aql::{BitmapHandle, CellTypeId, MemoryType, ScopeId, StatusId};
use cortex_core::memtable::CellVersion;
use cortex_core::CellDescriptor;

use crate::search::tokenize;
use crate::source_trust::{parse_source_trust_class, SourceTrust, SourceTrustClass};

const SCOPE_NS: u64 = 0x1000_0000_0000_0000;
const STATUS_NS: u64 = 0x2000_0000_0000_0000;
const TYPE_NS: u64 = 0x3000_0000_0000_0000;
const MEMORY_NS: u64 = 0x4000_0000_0000_0000;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceRef {
    pub source_id: String,
    pub source_url: Option<String>,
    pub document_id: Option<String>,
    pub page: Option<u32>,
    pub row: Option<u32>,
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
    pub source_trust_class: Option<SourceTrustClass>,
    pub source: Option<String>,
    pub citation: Option<String>,
    pub title: Option<String>,
    pub content_hash: Option<String>,
    pub source_hash: Option<String>,
    pub document_id: Option<String>,
    pub chunk_id: Option<String>,
    pub parent_id: Option<String>,
    pub chunk_role: Option<String>,
    pub path: Option<String>,
    pub section: Option<String>,
    pub project: Option<String>,
    pub entity: Option<String>,
    pub sector: Option<String>,
    pub owner: Option<String>,
    pub status_tag: Option<String>,
    pub event_date: Option<String>,
    pub topic: Option<String>,
    pub as_of: Option<String>,
    pub valid_from: Option<String>,
    pub valid_to: Option<String>,
    pub supersedes: Option<String>,
    pub superseded_by: Option<String>,
    pub table_id: Option<String>,
    pub table_headers: Option<String>,
    pub row_label: Option<String>,
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
        let mut source_trust_class = None;
        let mut source = None;
        let mut citation = None;
        let mut title = None;
        let mut content_hash = None;
        let mut source_hash = None;
        let mut document_id_field = None;
        let mut chunk_id = None;
        let mut parent_id = None;
        let mut chunk_role = None;
        let mut path = None;
        let mut section = None;
        let mut project = None;
        let mut entity = None;
        let mut sector = None;
        let mut owner = None;
        let mut status_tag = None;
        let mut event_date = None;
        let mut topic = None;
        let mut as_of = None;
        let mut valid_from = None;
        let mut valid_to = None;
        let mut supersedes = None;
        let mut superseded_by = None;
        let mut table_id = None;
        let mut table_headers = None;
        let mut row_label = None;
        let mut body_lines = Vec::new();
        let mut in_header = true;

        let mut source_id_val = None;
        let mut source_url = None;
        let mut document_id = None;
        let mut page = None;
        let mut row = None;
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
                } else if let Some(value) = line.strip_prefix("source_trust_class=") {
                    source_trust_class = parse_source_trust_class(value);
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
                } else if let Some(value) = line.strip_prefix("content_hash=") {
                    content_hash = non_empty(value);
                    continue;
                } else if let Some(value) = line.strip_prefix("source_hash=") {
                    source_hash = non_empty(value);
                    continue;
                } else if let Some(value) = line.strip_prefix("document_id=") {
                    document_id_field = non_empty(value);
                    document_id = document_id_field.clone();
                    continue;
                } else if let Some(value) = line.strip_prefix("doc_id=") {
                    document_id_field = non_empty(value);
                    document_id = document_id_field.clone();
                    continue;
                } else if let Some(value) = line.strip_prefix("chunk_id=") {
                    chunk_id = non_empty(value);
                    cell_range = chunk_id.clone();
                    continue;
                } else if let Some(value) = line.strip_prefix("parent_id=") {
                    parent_id = non_empty(value);
                    continue;
                } else if let Some(value) = line.strip_prefix("parent_chunk_id=") {
                    parent_id = non_empty(value);
                    continue;
                } else if let Some(value) = line.strip_prefix("chunk_role=") {
                    chunk_role = non_empty(value);
                    continue;
                } else if let Some(value) = line.strip_prefix("chunk_kind=") {
                    chunk_role = non_empty(value);
                    continue;
                } else if let Some(value) = line.strip_prefix("path=") {
                    path = non_empty(value);
                    continue;
                } else if let Some(value) = line.strip_prefix("section=") {
                    section = non_empty(value);
                    continue;
                } else if let Some(value) = line.strip_prefix("project=") {
                    project = non_empty(value);
                    continue;
                } else if let Some(value) = line.strip_prefix("entity=") {
                    entity = non_empty(value);
                    continue;
                } else if let Some(value) = line.strip_prefix("sector=") {
                    sector = non_empty(value);
                    continue;
                } else if let Some(value) = line.strip_prefix("owner=") {
                    owner = non_empty(value);
                    continue;
                } else if let Some(value) = line.strip_prefix("status_tag=") {
                    status_tag = non_empty(value);
                    continue;
                } else if let Some(value) = line.strip_prefix("event_date=") {
                    event_date = non_empty(value);
                    continue;
                } else if let Some(value) = line.strip_prefix("topic=") {
                    topic = non_empty(value);
                    continue;
                } else if let Some(value) = line.strip_prefix("as_of=") {
                    as_of = non_empty(value);
                    continue;
                } else if let Some(value) = line.strip_prefix("valid_from=") {
                    valid_from = non_empty(value);
                    continue;
                } else if let Some(value) = line.strip_prefix("valid_to=") {
                    valid_to = non_empty(value);
                    continue;
                } else if let Some(value) = line.strip_prefix("supersedes=") {
                    supersedes = non_empty(value);
                    continue;
                } else if let Some(value) = line.strip_prefix("superseded_by=") {
                    superseded_by = non_empty(value);
                    continue;
                } else if let Some(value) = line.strip_prefix("table_id=") {
                    table_id = non_empty(value);
                    continue;
                } else if let Some(value) = line.strip_prefix("table_headers=") {
                    table_headers = non_empty(value);
                    continue;
                } else if let Some(value) = line.strip_prefix("columns=") {
                    table_headers = non_empty(value);
                    continue;
                } else if let Some(value) = line.strip_prefix("row_label=") {
                    row_label = non_empty(value);
                    continue;
                } else if line.strip_prefix("embedding_model=").is_some()
                    || line.strip_prefix("embedding_text_hash=").is_some()
                    || line.strip_prefix("vector=").is_some()
                    || line.contains("_vector=")
                {
                    continue;
                } else if let Some(value) = line.strip_prefix("source_id=") {
                    source_id_val = non_empty(value);
                    continue;
                } else if let Some(value) = line.strip_prefix("source_url=") {
                    source_url = non_empty(value);
                    continue;
                } else if let Some(value) = line.strip_prefix("url=") {
                    source_url = non_empty(value);
                    continue;
                } else if let Some(value) = line.strip_prefix("page=") {
                    page = value.trim().parse().ok();
                    continue;
                } else if let Some(value) = line.strip_prefix("row=") {
                    row = value.trim().parse().ok();
                    continue;
                } else if let Some(value) = line.strip_prefix("row_number=") {
                    row = value.trim().parse().ok();
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
            source_url,
            document_id,
            page,
            row,
            cell_range,
            json_path,
            confidence_q16: confidence_q16.unwrap_or_else(|| {
                SourceTrust::from_metadata(source_trust_q16, source_trust_class).q16
            }),
        });

        Self {
            scope,
            status,
            cell_type,
            memory_type,
            ttl_seconds,
            created_unix_seconds,
            source_trust_q16,
            source_trust_class,
            source,
            citation,
            title,
            content_hash,
            source_hash,
            document_id: document_id_field,
            chunk_id,
            parent_id,
            chunk_role,
            path,
            section,
            project,
            entity,
            sector,
            owner,
            status_tag,
            event_date,
            topic,
            as_of,
            valid_from,
            valid_to,
            supersedes,
            superseded_by,
            table_id,
            table_headers,
            row_label,
            body_text,
            terms,
            source_ref,
        }
    }

    pub fn from_payload_with_descriptor(payload: &[u8], descriptor: &CellDescriptor) -> Self {
        let mut metadata = Self::from_payload(payload);
        metadata.scope = descriptor.scope.clone();
        metadata.status = descriptor.status.clone();
        metadata.cell_type = descriptor.cell_type.as_str().to_owned();
        metadata.memory_type = descriptor
            .memory_type
            .as_deref()
            .and_then(|value| value.parse().ok());
        metadata.ttl_seconds = descriptor.ttl_seconds;
        metadata.created_unix_seconds = descriptor.created_unix_seconds;
        metadata.source_trust_q16 = descriptor.source_trust_q16;
        metadata.source = descriptor.source.clone();
        metadata.citation = descriptor.citation.clone();
        metadata.content_hash = descriptor.content_hash.clone();
        metadata.parent_id = descriptor.parent_id.clone();
        metadata.valid_from = descriptor.valid_from.clone();
        metadata.valid_to = descriptor.valid_to.clone();
        let legacy_source_ref = metadata.source_ref.take();
        let source_id = metadata
            .source
            .clone()
            .or_else(|| metadata.citation.clone())
            .or_else(|| {
                legacy_source_ref
                    .as_ref()
                    .map(|source| source.source_id.clone())
            });
        metadata.source_ref = source_id.map(|source_id| {
            let source_trust =
                SourceTrust::from_metadata(metadata.source_trust_q16, metadata.source_trust_class);
            SourceRef {
                source_id,
                source_url: legacy_source_ref
                    .as_ref()
                    .and_then(|source| source.source_url.clone()),
                document_id: legacy_source_ref
                    .as_ref()
                    .and_then(|source| source.document_id.clone()),
                page: legacy_source_ref.as_ref().and_then(|source| source.page),
                row: legacy_source_ref.as_ref().and_then(|source| source.row),
                cell_range: legacy_source_ref
                    .as_ref()
                    .and_then(|source| source.cell_range.clone()),
                json_path: legacy_source_ref
                    .as_ref()
                    .and_then(|source| source.json_path.clone()),
                confidence_q16: source_trust.q16,
            }
        });
        metadata
    }

    pub fn from_version(version: &CellVersion) -> Self {
        Self::from_payload_with_descriptor(&version.payload, &version.descriptor)
    }

    pub fn citation(&self) -> Option<&str> {
        self.citation
            .as_deref()
            .or(self.source.as_deref())
            .or_else(|| {
                self.source_ref
                    .as_ref()
                    .map(|source_ref| source_ref.source_id.as_str())
            })
    }

    pub fn weighted_lexical_terms(&self) -> BTreeMap<String, u32> {
        let mut terms = BTreeMap::new();
        for (field, field_terms) in self.lexical_field_terms() {
            let weight = lexical_field_weight(&field);
            for (term, frequency) in field_terms {
                *terms.entry(term).or_default() += frequency.saturating_mul(weight);
            }
        }
        terms
    }

    pub fn lexical_field_terms(&self) -> BTreeMap<String, BTreeMap<String, u32>> {
        let mut fields = BTreeMap::new();
        add_field_terms(&mut fields, "body", &self.body_text);
        if let Some(title) = &self.title {
            add_field_terms(&mut fields, "title", title);
        }
        for value in [
            self.path.as_ref(),
            self.document_id.as_ref(),
            self.section.as_ref(),
        ]
        .into_iter()
        .flatten()
        {
            add_field_terms(&mut fields, "path", value);
        }
        for value in [
            self.project.as_ref(),
            self.entity.as_ref(),
            self.sector.as_ref(),
            self.owner.as_ref(),
            self.status_tag.as_ref(),
            self.event_date.as_ref(),
            self.topic.as_ref(),
            self.as_of.as_ref(),
            self.valid_from.as_ref(),
            self.valid_to.as_ref(),
            self.source.as_ref(),
        ]
        .into_iter()
        .flatten()
        {
            add_field_terms(&mut fields, "entity", value);
        }
        if let Some(chunk_id) = &self.chunk_id {
            add_field_terms(&mut fields, "chunk", chunk_id);
        }
        if let Some(parent_id) = &self.parent_id {
            add_field_terms(&mut fields, "chunk", parent_id);
        }
        for value in [
            self.table_id.as_ref(),
            self.table_headers.as_ref(),
            self.row_label.as_ref(),
            self.source_ref
                .as_ref()
                .and_then(|source| source.cell_range.as_ref()),
        ]
        .into_iter()
        .flatten()
        {
            add_field_terms(&mut fields, "table", value);
        }
        fields
    }
}

pub(crate) fn non_empty(value: &str) -> Option<String> {
    let value = value.trim();
    (!value.is_empty()).then(|| value.to_owned())
}

pub(crate) fn lexical_field_weight(field: &str) -> u32 {
    match field {
        "title" => 8,
        "table" => 6,
        "path" => 5,
        "entity" => 4,
        "chunk" => 2,
        _ => 1,
    }
}

fn add_field_terms(fields: &mut BTreeMap<String, BTreeMap<String, u32>>, field: &str, text: &str) {
    for term in tokenize(text) {
        *fields
            .entry(field.to_owned())
            .or_default()
            .entry(term)
            .or_default() += 1;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weighted_lexical_terms_include_document_views() {
        let metadata = CellMetadata::from_payload(
            b"scope=docs\nstatus=ready\ntitle=Payments Migration\npath=confluence/payments/runbook\ndocument_id=doc-payments\nchunk_id=chunk-7\nparent_id=chunk-parent\nchunk_role=child\nsection=Rollout Plan\nproject=Apollo\nentity=Payments API\nsector=platform\nsource=confluence\n\nbody mentions payments once",
        );
        let terms = metadata.weighted_lexical_terms();

        assert_eq!(metadata.parent_id.as_deref(), Some("chunk-parent"));
        assert_eq!(metadata.chunk_role.as_deref(), Some("child"));
        assert!(terms.get("migration").copied().unwrap_or(0) >= 8);
        assert!(terms.get("runbook").copied().unwrap_or(0) >= 5);
        assert!(terms.get("apollo").copied().unwrap_or(0) >= 4);
        assert!(terms.get("chunk").copied().unwrap_or(0) >= 2);
    }

    #[test]
    fn embedding_lines_do_not_pollute_body_text() {
        let metadata = CellMetadata::from_payload(
            b"scope=docs\nstatus=ready\nembedding_model=bge-m3\nembedding_text_hash=abc\nvector=1,2,3\ntitle_vector=3,2,1\n\nAlpha body",
        );

        assert_eq!(metadata.body_text, "Alpha body");
        assert!(!metadata.terms.contains(&"vector".to_owned()));
        assert!(!metadata.terms.contains(&"bge".to_owned()));
        assert!(!metadata.terms.contains(&"title".to_owned()));
    }

    #[test]
    fn weighted_lexical_terms_include_table_views() {
        let metadata = CellMetadata::from_payload(
            b"scope=docs\nstatus=ready\ntype=table\nsource=csv\ntable_id=budget.csv\ntable_headers=project|budget|owner\nrow_label=Apollo\ncell_range=row-7\n\nproject: Apollo\nbudget: 12000",
        );
        let terms = metadata.weighted_lexical_terms();

        assert_eq!(metadata.cell_type, "table");
        assert_eq!(metadata.table_id.as_deref(), Some("budget.csv"));
        assert_eq!(metadata.row_label.as_deref(), Some("Apollo"));
        assert!(terms.get("budget").copied().unwrap_or(0) >= 6);
        assert!(terms.get("apollo").copied().unwrap_or(0) >= 6);
        assert!(terms.get("row").copied().unwrap_or(0) >= 6);
    }

    #[test]
    fn weighted_lexical_terms_include_enrichment_views() {
        let metadata = CellMetadata::from_payload(
            b"scope=docs\nstatus=ready\nproject=Apollo\nowner=Alice Lee\nstatus_tag=blocked\nevent_date=2026-05-14\ntopic=Migration Runbook\n\nbody",
        );
        let terms = metadata.weighted_lexical_terms();

        assert_eq!(metadata.project.as_deref(), Some("Apollo"));
        assert_eq!(metadata.owner.as_deref(), Some("Alice Lee"));
        assert_eq!(metadata.status_tag.as_deref(), Some("blocked"));
        assert_eq!(metadata.event_date.as_deref(), Some("2026-05-14"));
        assert_eq!(metadata.topic.as_deref(), Some("Migration Runbook"));
        assert!(terms.get("alice").copied().unwrap_or(0) >= 4);
        assert!(terms.get("blocked").copied().unwrap_or(0) >= 4);
        assert!(terms.get("migration").copied().unwrap_or(0) >= 4);
    }
}
