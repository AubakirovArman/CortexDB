use cortex_core::{CellId, CommitSeq, KnowledgeCell};

use crate::database::Database;
use crate::embedding_pipeline::Embedder;
use crate::error::{EngineError, EngineResult};
use crate::ingestion::cells::{
    entity_metadata, fact_metadata, offset_cell_id, put_source_ref_cell, put_text_chunk_cell,
    relation_metadata, table_metadata, SourceRefHeaders,
};
use crate::ingestion::chunking::{split_text_chunks, TableChunkPolicy, TextChunkPolicy};
use crate::ingestion::dedup::{content_hash_hex, source_hash_hex};
use crate::ingestion::formats::{csv_rows, flat_json_fields};
use crate::ingestion::IngestionUpdatePolicy;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IngestedCell {
    pub cell_id: CellId,
    pub commit_seq: CommitSeq,
    pub chunk_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextIngestOptions {
    pub scope: String,
    pub source: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JsonIngestOptions {
    pub scope: String,
    pub source: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CsvIngestOptions {
    pub scope: String,
    pub source: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PdfIngestOptions {
    pub scope: String,
    pub source: String,
    pub page: Option<u32>,
}

impl Database {
    pub fn ingest_text(
        &mut self,
        cell_id: CellId,
        text: &str,
        options: TextIngestOptions,
    ) -> EngineResult<IngestedCell> {
        let results = self.ingest_text_chunks(cell_id, text, options)?;
        results
            .first()
            .cloned()
            .ok_or(EngineError::InvalidOperation)
    }

    pub fn ingest_text_chunks(
        &mut self,
        first_cell_id: CellId,
        text: &str,
        options: TextIngestOptions,
    ) -> EngineResult<Vec<IngestedCell>> {
        self.ingest_text_chunks_with_policy(
            first_cell_id,
            text,
            options,
            TextChunkPolicy::default(),
        )
    }

    pub fn ingest_text_chunks_with_policy(
        &mut self,
        first_cell_id: CellId,
        text: &str,
        options: TextIngestOptions,
        policy: TextChunkPolicy,
    ) -> EngineResult<Vec<IngestedCell>> {
        self.ingest_text_chunks_with_update_policy(
            first_cell_id,
            text,
            options,
            policy,
            IngestionUpdatePolicy::AlwaysInsert,
        )
    }

    pub fn ingest_text_chunks_with_update_policy(
        &mut self,
        first_cell_id: CellId,
        text: &str,
        options: TextIngestOptions,
        policy: TextChunkPolicy,
        update_policy: IngestionUpdatePolicy,
    ) -> EngineResult<Vec<IngestedCell>> {
        self.ingest_text_chunks_inner(first_cell_id, text, options, policy, update_policy, None)
    }

    /// Ingest text chunks and embed each chunk body through `embedder`, writing a
    /// `vector=` header into each cell so it becomes retrievable by vector search.
    /// The engine stays network-free: the embedding backend is injected by the
    /// caller (e.g. the server, which owns the HTTP client). An embedding error
    /// fails the ingest call (fail-closed).
    pub fn ingest_text_chunks_with_embedder(
        &mut self,
        first_cell_id: CellId,
        text: &str,
        options: TextIngestOptions,
        policy: TextChunkPolicy,
        update_policy: IngestionUpdatePolicy,
        embedder: &dyn Embedder,
    ) -> EngineResult<Vec<IngestedCell>> {
        self.ingest_text_chunks_inner(
            first_cell_id,
            text,
            options,
            policy,
            update_policy,
            Some(embedder),
        )
    }

    fn ingest_text_chunks_inner(
        &mut self,
        first_cell_id: CellId,
        text: &str,
        options: TextIngestOptions,
        policy: TextChunkPolicy,
        update_policy: IngestionUpdatePolicy,
        embedder: Option<&dyn Embedder>,
    ) -> EngineResult<Vec<IngestedCell>> {
        let chunks = split_text_chunks(&options.source, text, policy)?;
        if chunks.is_empty() {
            return Ok(Vec::new());
        }

        let mut ingested = Vec::new();
        for (index, chunk) in chunks.iter().enumerate() {
            let source_hash = source_hash_hex(&options.source);
            let content_hash = content_hash_hex(&chunk.text);
            if update_policy == IngestionUpdatePolicy::SkipExisting
                && self
                    .find_duplicate_ingestion_chunk(&source_hash, &content_hash)
                    .is_some()
            {
                continue;
            }
            let cell_id = offset_cell_id(first_cell_id, index)?;
            let vector = match embedder {
                Some(embedder) => Some(embedder.embed(&chunk.text)?),
                None => None,
            };
            let commit_seq = put_text_chunk_cell(
                self,
                cell_id,
                chunk,
                &options.scope,
                &options.source,
                vector.as_deref(),
            )?;
            ingested.push(IngestedCell {
                cell_id,
                commit_seq,
                chunk_id: Some(chunk.chunk_id.clone()),
            });
        }
        Ok(ingested)
    }

    pub fn ingest_json(
        &mut self,
        first_cell_id: CellId,
        json: &str,
        options: JsonIngestOptions,
    ) -> EngineResult<Vec<IngestedCell>> {
        flat_json_fields(json)?
            .into_iter()
            .enumerate()
            .map(|(index, (key, value))| {
                let cell_id = offset_cell_id(first_cell_id, index)?;
                let body = format!("{key}: {value}");
                let commit_seq = put_source_ref_cell(
                    self,
                    cell_id,
                    fact_metadata(options.scope.clone(), options.source.clone()),
                    &body,
                    SourceRefHeaders {
                        document_id: &options.source,
                        page: None,
                        row: None,
                        cell_range: None,
                        json_path: Some(&key),
                        confidence_q16: None,
                        extra_headers: &[],
                    },
                )?;
                Ok(IngestedCell {
                    cell_id,
                    commit_seq,
                    chunk_id: None,
                })
            })
            .collect()
    }

    pub fn ingest_csv(
        &mut self,
        first_cell_id: CellId,
        csv: &str,
        options: CsvIngestOptions,
    ) -> EngineResult<Vec<IngestedCell>> {
        let rows = csv_rows(csv)?;
        let Some(headers) = rows.first() else {
            return Ok(Vec::new());
        };
        let table_policy = TableChunkPolicy::default().validate()?;
        rows.iter()
            .skip(1)
            .enumerate()
            .map(|(index, row)| {
                let cell_id = offset_cell_id(first_cell_id, index)?;
                let source_row = table_policy.source_row_number(index)?;
                let cell_range = table_policy.cell_range(source_row);
                let body = headers
                    .iter()
                    .zip(row)
                    .map(|(header, value)| format!("{header}: {value}"))
                    .collect::<Vec<_>>()
                    .join("\n");
                let extra_headers = vec![
                    ("table_id", options.source.clone()),
                    ("table_headers", headers.join("|")),
                    ("row_label", row.first().cloned().unwrap_or_default()),
                    ("chunk_role", "table_row".to_owned()),
                ];
                let commit_seq = put_source_ref_cell(
                    self,
                    cell_id,
                    table_metadata(options.scope.clone(), options.source.clone()),
                    &body,
                    SourceRefHeaders {
                        document_id: &options.source,
                        page: None,
                        row: Some(source_row),
                        cell_range: Some(&cell_range),
                        json_path: None,
                        confidence_q16: None,
                        extra_headers: &extra_headers,
                    },
                )?;
                Ok(IngestedCell {
                    cell_id,
                    commit_seq,
                    chunk_id: None,
                })
            })
            .collect()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EntityIngestOptions {
    pub scope: String,
    pub source: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RelationIngestOptions {
    pub scope: String,
    pub source: String,
}

impl Database {
    pub fn ingest_entity(
        &mut self,
        cell_id: CellId,
        body: &str,
        options: EntityIngestOptions,
    ) -> EngineResult<IngestedCell> {
        let metadata = entity_metadata(options.scope, options.source);
        let cell = KnowledgeCell::new(metadata, body);
        let seq = self.put_knowledge_cell(cell_id, cell)?;
        Ok(IngestedCell {
            cell_id,
            commit_seq: seq,
            chunk_id: None,
        })
    }

    pub fn ingest_relation(
        &mut self,
        cell_id: CellId,
        body: &str,
        options: RelationIngestOptions,
    ) -> EngineResult<IngestedCell> {
        let metadata = relation_metadata(options.scope, options.source);
        let cell = KnowledgeCell::new(metadata, body);
        let seq = self.put_knowledge_cell(cell_id, cell)?;
        Ok(IngestedCell {
            cell_id,
            commit_seq: seq,
            chunk_id: None,
        })
    }
}
