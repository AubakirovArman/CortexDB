use cortex_core::{CellId, CommitSeq, KnowledgeCellMetadata, KnowledgeCellType};

use crate::database::Database;
use crate::error::{EngineError, EngineResult};
use crate::ingestion::chunking::{sanitize_header_value, TextChunk};
use crate::ingestion::dedup::{content_hash_hex, source_hash_hex};
use crate::ingestion::enrichment::SourceMetadataEnrichment;
use crate::operation::DbOperation;

pub(crate) fn put_text_chunk_cell(
    db: &mut Database,
    cell_id: CellId,
    chunk: &TextChunk,
    scope: &str,
    source: &str,
    vector: Option<&[i16]>,
) -> EngineResult<CommitSeq> {
    let metadata = document_metadata(scope.to_owned(), source.to_owned());
    let payload = text_chunk_payload(&metadata, source, chunk, vector);
    db.append_then_apply_with_metadata(
        DbOperation::PutCell { cell_id, payload },
        metadata.encode_wal_section(),
    )
}

pub(crate) struct SourceRefHeaders<'a> {
    pub document_id: &'a str,
    pub page: Option<u32>,
    pub row: Option<u32>,
    pub cell_range: Option<&'a str>,
    pub json_path: Option<&'a str>,
    pub confidence_q16: Option<u16>,
    pub extra_headers: &'a [(&'a str, String)],
}

pub(crate) fn put_source_ref_cell(
    db: &mut Database,
    cell_id: CellId,
    metadata: KnowledgeCellMetadata,
    body: &str,
    source_ref: SourceRefHeaders<'_>,
) -> EngineResult<CommitSeq> {
    let wal_metadata = metadata.encode_wal_section();
    let payload = source_ref_payload(&metadata, body, source_ref);
    db.append_then_apply_with_metadata(DbOperation::PutCell { cell_id, payload }, wal_metadata)
}

pub(crate) fn document_metadata(scope: String, source: String) -> KnowledgeCellMetadata {
    KnowledgeCellMetadata {
        scope,
        status: "ready".to_owned(),
        cell_type: KnowledgeCellType::DocumentBlock,
        source: Some(source),
        ..KnowledgeCellMetadata::default()
    }
}

pub(crate) fn table_metadata(scope: String, source: String) -> KnowledgeCellMetadata {
    KnowledgeCellMetadata {
        scope,
        status: "ready".to_owned(),
        cell_type: KnowledgeCellType::Table,
        source: Some(source),
        ..KnowledgeCellMetadata::default()
    }
}

pub(crate) fn fact_metadata(scope: String, source: String) -> KnowledgeCellMetadata {
    KnowledgeCellMetadata {
        scope,
        status: "ready".to_owned(),
        cell_type: KnowledgeCellType::Fact,
        source: Some(source),
        ..KnowledgeCellMetadata::default()
    }
}

pub(crate) fn entity_metadata(scope: String, source: String) -> KnowledgeCellMetadata {
    KnowledgeCellMetadata {
        scope,
        status: "ready".to_owned(),
        cell_type: KnowledgeCellType::Entity,
        source: Some(source),
        ..KnowledgeCellMetadata::default()
    }
}

pub(crate) fn relation_metadata(scope: String, source: String) -> KnowledgeCellMetadata {
    KnowledgeCellMetadata {
        scope,
        status: "ready".to_owned(),
        cell_type: KnowledgeCellType::Relation,
        source: Some(source),
        ..KnowledgeCellMetadata::default()
    }
}

pub(crate) fn offset_cell_id(first: CellId, offset: usize) -> EngineResult<CellId> {
    let offset = u64::try_from(offset)
        .map_err(|_| EngineError::StorageInvariant("ingestion batch is too large".to_owned()))?;
    first
        .0
        .checked_add(offset)
        .map(CellId)
        .ok_or_else(|| EngineError::StorageInvariant("cell id overflow".to_owned()))
}

fn text_chunk_payload(
    metadata: &KnowledgeCellMetadata,
    document_id: &str,
    chunk: &TextChunk,
    vector: Option<&[i16]>,
) -> Vec<u8> {
    let mut lines = vec![
        format!("scope={}", sanitize_header_value(&metadata.scope)),
        format!("status={}", sanitize_header_value(&metadata.status)),
        format!("type={}", metadata.cell_type.as_str()),
    ];
    if let Some(source) = &metadata.source {
        let source = sanitize_header_value(source);
        lines.push(format!("source={source}"));
        lines.push(format!("source_id={source}"));
    }
    lines.push(format!("source_hash={}", source_hash_hex(document_id)));
    lines.push(format!("content_hash={}", content_hash_hex(&chunk.text)));
    lines.push(format!(
        "document_id={}",
        sanitize_header_value(document_id)
    ));
    lines.push(format!(
        "chunk_id={}",
        sanitize_header_value(&chunk.chunk_id)
    ));
    push_enrichment_headers(&mut lines, &chunk.text, document_id);
    if let Some(vector) = vector.filter(|values| !values.is_empty()) {
        lines.push(format!("vector={}", vector_literal(vector)));
    }
    lines.push(String::new());
    let mut payload = lines.join("\n").into_bytes();
    payload.extend_from_slice(chunk.text.as_bytes());
    payload
}

fn vector_literal(vector: &[i16]) -> String {
    vector
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(",")
}

fn source_ref_payload(
    metadata: &KnowledgeCellMetadata,
    body: &str,
    source_ref: SourceRefHeaders<'_>,
) -> Vec<u8> {
    let mut lines = base_header_lines(metadata);
    if let Some(source) = &metadata.source {
        let source = sanitize_header_value(source);
        lines.push(format!("source_id={source}"));
        lines.push(format!("source_hash={}", source_hash_hex(&source)));
    }
    lines.push(format!("content_hash={}", content_hash_hex(body)));
    lines.push(format!(
        "document_id={}",
        sanitize_header_value(source_ref.document_id)
    ));
    if let Some(page) = source_ref.page {
        lines.push(format!("page={page}"));
    }
    if let Some(row) = source_ref.row {
        lines.push(format!("row={row}"));
    }
    if let Some(cell_range) = source_ref.cell_range {
        lines.push(format!("cell_range={}", sanitize_header_value(cell_range)));
    }
    if let Some(json_path) = source_ref.json_path {
        lines.push(format!("json_path={}", sanitize_header_value(json_path)));
    }
    if let Some(confidence_q16) = source_ref.confidence_q16 {
        lines.push(format!("confidence_q16={confidence_q16}"));
    }
    push_enrichment_headers(&mut lines, body, source_ref.document_id);
    for (key, value) in source_ref.extra_headers {
        lines.push(format!(
            "{}={}",
            sanitize_header_value(key),
            sanitize_header_value(value)
        ));
    }
    lines.push(String::new());
    let mut payload = lines.join("\n").into_bytes();
    payload.extend_from_slice(body.as_bytes());
    payload
}

fn base_header_lines(metadata: &KnowledgeCellMetadata) -> Vec<String> {
    let mut lines = vec![
        format!("scope={}", sanitize_header_value(&metadata.scope)),
        format!("status={}", sanitize_header_value(&metadata.status)),
        format!("type={}", metadata.cell_type.as_str()),
    ];
    if let Some(source) = &metadata.source {
        lines.push(format!("source={}", sanitize_header_value(source)));
    }
    lines
}

fn push_enrichment_headers(lines: &mut Vec<String>, body: &str, source: &str) {
    for (key, value) in SourceMetadataEnrichment::from_text(body, source).header_pairs() {
        lines.push(format!("{key}={value}"));
    }
}
