use cortex_core::{CellId, CommitSeq, KnowledgeCell};

use crate::database::Database;
use crate::error::{EngineError, EngineResult};
use crate::ingestion::cells::{
    document_metadata, entity_metadata, fact_metadata, offset_cell_id, put_source_ref_cell,
    put_text_chunk_cell, relation_metadata, SourceRefHeaders,
};
use crate::ingestion::chunking::{split_text_chunks, TextChunkPolicy};
use crate::ingestion::extract_pdf_text;
use crate::ingestion::formats::{csv_rows, flat_json_fields};

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
        let chunks = split_text_chunks(&options.source, text, policy)?;
        if chunks.is_empty() {
            return Ok(Vec::new());
        }

        let mut ingested = Vec::new();
        for (index, chunk) in chunks.iter().enumerate() {
            let cell_id = offset_cell_id(first_cell_id, index)?;
            let commit_seq =
                put_text_chunk_cell(self, cell_id, chunk, &options.scope, &options.source)?;
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
        rows.iter()
            .skip(1)
            .enumerate()
            .map(|(index, row)| {
                let cell_id = offset_cell_id(first_cell_id, index)?;
                let source_row = u32::try_from(index + 2)
                    .map_err(|_| EngineError::StorageInvariant("csv row overflow".to_owned()))?;
                let cell_range = format!("row-{source_row}");
                let body = headers
                    .iter()
                    .zip(row)
                    .map(|(header, value)| format!("{header}: {value}"))
                    .collect::<Vec<_>>()
                    .join("\n");
                let commit_seq = put_source_ref_cell(
                    self,
                    cell_id,
                    document_metadata(options.scope.clone(), options.source.clone()),
                    &body,
                    SourceRefHeaders {
                        document_id: &options.source,
                        page: None,
                        row: Some(source_row),
                        cell_range: Some(&cell_range),
                        json_path: None,
                        confidence_q16: None,
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

    pub fn ingest_pdf_text(
        &mut self,
        cell_id: CellId,
        extracted_text: &str,
        options: PdfIngestOptions,
    ) -> EngineResult<IngestedCell> {
        let body = format!("source_format=pdf\n{extracted_text}");
        let source = options.source;
        let commit_seq = put_source_ref_cell(
            self,
            cell_id,
            document_metadata(options.scope, source.clone()),
            &body,
            SourceRefHeaders {
                document_id: &source,
                page: options.page,
                row: None,
                cell_range: None,
                json_path: None,
                confidence_q16: None,
            },
        )?;
        Ok(IngestedCell {
            cell_id,
            commit_seq,
            chunk_id: None,
        })
    }

    pub fn ingest_pdf_bytes(
        &mut self,
        cell_id: CellId,
        pdf: &[u8],
        options: PdfIngestOptions,
    ) -> EngineResult<IngestedCell> {
        let extracted = extract_pdf_text(pdf)?;
        self.ingest_pdf_text(cell_id, &extracted.text, options)
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
