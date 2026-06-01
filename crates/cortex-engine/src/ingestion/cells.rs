use cortex_core::{CellId, CommitSeq, KnowledgeCellMetadata, KnowledgeCellType};

use crate::database::Database;
use crate::error::{EngineError, EngineResult};
use crate::ingestion::chunking::{sanitize_header_value, TextChunk};
use crate::operation::DbOperation;

pub(crate) fn put_text_chunk_cell(
    db: &mut Database,
    cell_id: CellId,
    chunk: &TextChunk,
    scope: &str,
    source: &str,
) -> EngineResult<CommitSeq> {
    let metadata = document_metadata(scope.to_owned(), source.to_owned());
    let payload = text_chunk_payload(&metadata, source, chunk);
    db.append_then_apply_with_metadata(
        DbOperation::PutCell { cell_id, payload },
        metadata.encode_wal_section(),
    )
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
    lines.push(format!(
        "document_id={}",
        sanitize_header_value(document_id)
    ));
    lines.push(format!(
        "chunk_id={}",
        sanitize_header_value(&chunk.chunk_id)
    ));
    lines.push(String::new());
    let mut payload = lines.join("\n").into_bytes();
    payload.extend_from_slice(chunk.text.as_bytes());
    payload
}
