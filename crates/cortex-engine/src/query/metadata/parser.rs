use cortex_core::memtable::CellVersion;
use cortex_core::{CellDescriptor, CellId};

use crate::search::tokenize;
use crate::source_trust::{parse_source_trust_class, SourceTrust};

use super::fields::non_empty;
use super::types::{CellMetadata, SourceRef};

#[cfg(test)]
use std::cell::Cell as TestCell;

#[cfg(test)]
thread_local! {
    static LEGACY_PAYLOAD_METADATA_PARSE_CALLS: TestCell<usize> = const { TestCell::new(0) };
}

impl CellMetadata {
    pub fn from_payload(payload: &[u8]) -> Self {
        #[cfg(test)]
        LEGACY_PAYLOAD_METADATA_PARSE_CALLS.with(|calls| calls.set(calls.get() + 1));
        Self::from_payload_inner(payload, None)
    }

    fn from_payload_inner(payload: &[u8], descriptor: Option<&CellDescriptor>) -> Self {
        let descriptor_backed = descriptor.is_some();
        let text = String::from_utf8_lossy(payload);
        let mut scope = descriptor
            .map(|value| value.scope.clone())
            .unwrap_or_else(|| "default".to_owned());
        let mut status = descriptor
            .map(|value| value.status.clone())
            .unwrap_or_else(|| "ready".to_owned());
        let mut cell_type = descriptor
            .map(|value| value.cell_type.as_str().to_owned())
            .unwrap_or_else(|| "raw".to_owned());
        let mut memory_type = descriptor
            .and_then(|value| value.memory_type.as_deref())
            .and_then(|value| value.parse().ok());
        let mut ttl_seconds = descriptor.and_then(|value| value.ttl_seconds);
        let mut created_unix_seconds = descriptor.and_then(|value| value.created_unix_seconds);
        let mut source_trust_q16 = descriptor.and_then(|value| value.source_trust_q16);
        let mut source_trust_class = None;
        let mut source = descriptor.and_then(|value| value.source.clone());
        let mut citation = descriptor.and_then(|value| value.citation.clone());
        let mut title = None;
        let mut content_hash = descriptor.and_then(|value| value.content_hash.clone());
        let mut source_hash = None;
        let mut document_id_field = descriptor.and_then(|value| value.document_id.clone());
        let mut chunk_id = None;
        let mut parent_id = descriptor.and_then(|value| value.parent_id.clone());
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
        let mut valid_from = descriptor.and_then(|value| value.valid_from.clone());
        let mut valid_to = descriptor.and_then(|value| value.valid_to.clone());
        let mut supersedes = None;
        let mut superseded_by = None;
        let mut compression_kind = None;
        let mut compression_source_cells = Vec::new();
        let mut compression_answerability_q16 = None;
        let mut compression_worker = None;
        let mut table_id = None;
        let mut table_headers = None;
        let mut row_label = None;
        let mut body_lines = Vec::new();
        let mut in_header = true;
        let mut source_id_val = descriptor.and_then(|value| value.source_id.clone());
        let mut source_url = descriptor.and_then(|value| value.source_url.clone());
        let mut document_id = descriptor.and_then(|value| value.document_id.clone());
        let mut page = descriptor.and_then(|value| value.page);
        let mut row = descriptor.and_then(|value| value.row);
        let mut cell_range = descriptor.and_then(|value| value.cell_range.clone());
        let mut json_path = descriptor.and_then(|value| value.json_path.clone());
        let mut confidence_q16 = descriptor.and_then(|value| value.confidence_q16);

        for line in text.lines() {
            if in_header {
                if line.trim().is_empty() {
                    in_header = false;
                    continue;
                }
                if let Some(value) = line.strip_prefix("scope=") {
                    if !descriptor_backed {
                        scope = value.trim().to_owned();
                    }
                    continue;
                } else if let Some(value) = line.strip_prefix("status=") {
                    if !descriptor_backed {
                        status = value.trim().to_owned();
                    }
                    continue;
                } else if let Some(value) = line.strip_prefix("type=") {
                    if !descriptor_backed {
                        cell_type = value.trim().to_owned();
                    }
                    continue;
                } else if let Some(value) = line.strip_prefix("memory_type=") {
                    if !descriptor_backed {
                        memory_type = value.trim().parse().ok();
                    }
                    continue;
                } else if let Some(value) = line.strip_prefix("ttl_seconds=") {
                    if !descriptor_backed {
                        ttl_seconds = value.trim().parse().ok();
                    }
                    continue;
                } else if let Some(value) = line.strip_prefix("created_unix_seconds=") {
                    if !descriptor_backed {
                        created_unix_seconds = value.trim().parse().ok();
                    }
                    continue;
                } else if let Some(value) = line.strip_prefix("source_trust_q16=") {
                    if !descriptor_backed {
                        source_trust_q16 = value.trim().parse().ok();
                    }
                    continue;
                } else if let Some(value) = line.strip_prefix("source_trust_class=") {
                    source_trust_class = parse_source_trust_class(value);
                    continue;
                } else if let Some(value) = line.strip_prefix("source=") {
                    if !descriptor_backed {
                        source = non_empty(value);
                    }
                    continue;
                } else if let Some(value) = line.strip_prefix("citation=") {
                    if !descriptor_backed {
                        citation = non_empty(value);
                    }
                    continue;
                } else if let Some(value) = line.strip_prefix("title=") {
                    title = non_empty(value);
                    continue;
                } else if let Some(value) = line.strip_prefix("content_hash=") {
                    if content_hash.is_none() {
                        content_hash = non_empty(value);
                    }
                    continue;
                } else if let Some(value) = line.strip_prefix("source_hash=") {
                    source_hash = non_empty(value);
                    continue;
                } else if let Some(value) = line.strip_prefix("document_id=") {
                    let parsed = non_empty(value);
                    document_id_field = document_id_field.or_else(|| parsed.clone());
                    document_id = document_id.or(parsed);
                    continue;
                } else if let Some(value) = line.strip_prefix("doc_id=") {
                    let parsed = non_empty(value);
                    document_id_field = document_id_field.or_else(|| parsed.clone());
                    document_id = document_id.or(parsed);
                    continue;
                } else if let Some(value) = line.strip_prefix("chunk_id=") {
                    chunk_id = non_empty(value);
                    cell_range = cell_range.or_else(|| chunk_id.clone());
                    continue;
                } else if let Some(value) = line.strip_prefix("parent_id=") {
                    if !descriptor_backed {
                        parent_id = non_empty(value);
                    }
                    continue;
                } else if let Some(value) = line.strip_prefix("parent_chunk_id=") {
                    if !descriptor_backed {
                        parent_id = non_empty(value);
                    }
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
                    if !descriptor_backed {
                        valid_from = non_empty(value);
                    }
                    continue;
                } else if let Some(value) = line.strip_prefix("valid_to=") {
                    if !descriptor_backed {
                        valid_to = non_empty(value);
                    }
                    continue;
                } else if let Some(value) = line.strip_prefix("supersedes=") {
                    supersedes = non_empty(value);
                    continue;
                } else if let Some(value) = line.strip_prefix("superseded_by=") {
                    superseded_by = non_empty(value);
                    continue;
                } else if let Some(value) = line.strip_prefix("compression_kind=") {
                    compression_kind = non_empty(value);
                    continue;
                } else if let Some(value) = line.strip_prefix("compression_source_cells=") {
                    compression_source_cells = parse_cell_id_list(value);
                    continue;
                } else if let Some(value) = line.strip_prefix("compression_answerability_q16=") {
                    compression_answerability_q16 = value.trim().parse().ok();
                    continue;
                } else if let Some(value) = line.strip_prefix("compression_worker=") {
                    compression_worker = non_empty(value);
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
                    || line.strip_prefix("embedding_ref=").is_some()
                    || line.strip_prefix("vector=").is_some()
                    || line.contains("_vector=")
                {
                    continue;
                } else if let Some(value) = line.strip_prefix("source_id=") {
                    if !descriptor_backed || source.is_none() {
                        source_id_val = source_id_val.or_else(|| non_empty(value));
                    }
                    continue;
                } else if let Some(value) = line.strip_prefix("source_url=") {
                    source_url = source_url.or_else(|| non_empty(value));
                    continue;
                } else if let Some(value) = line.strip_prefix("url=") {
                    source_url = source_url.or_else(|| non_empty(value));
                    continue;
                } else if let Some(value) = line.strip_prefix("page=") {
                    page = page.or_else(|| value.trim().parse().ok());
                    continue;
                } else if let Some(value) = line.strip_prefix("row=") {
                    row = row.or_else(|| value.trim().parse().ok());
                    continue;
                } else if let Some(value) = line.strip_prefix("row_number=") {
                    row = row.or_else(|| value.trim().parse().ok());
                    continue;
                } else if let Some(value) = line.strip_prefix("cell_range=") {
                    cell_range = cell_range.or_else(|| non_empty(value));
                    continue;
                } else if let Some(value) = line.strip_prefix("json_path=") {
                    json_path = json_path.or_else(|| non_empty(value));
                    continue;
                } else if let Some(value) = line.strip_prefix("confidence_q16=") {
                    confidence_q16 = confidence_q16.or_else(|| value.trim().parse().ok());
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
            compression_kind,
            compression_source_cells,
            compression_answerability_q16,
            compression_worker,
            table_id,
            table_headers,
            row_label,
            body_text,
            terms,
            source_ref,
        }
    }

    pub fn from_payload_with_descriptor(payload: &[u8], descriptor: &CellDescriptor) -> Self {
        Self::from_payload_inner(payload, Some(descriptor))
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
                    .map(|value| value.source_id.as_str())
            })
    }

    #[cfg(test)]
    pub(crate) fn reset_legacy_payload_metadata_parse_profile() {
        LEGACY_PAYLOAD_METADATA_PARSE_CALLS.with(|calls| calls.set(0));
    }

    #[cfg(test)]
    pub(crate) fn legacy_payload_metadata_parse_profile() -> usize {
        LEGACY_PAYLOAD_METADATA_PARSE_CALLS.with(TestCell::get)
    }
}

fn parse_cell_id_list(value: &str) -> Vec<CellId> {
    value
        .split(',')
        .filter_map(|item| item.trim().parse::<u64>().ok())
        .map(CellId)
        .collect()
}
