use cortex_core::{CellId, CommitSeq, KnowledgeCell, KnowledgeCellMetadata, KnowledgeCellType};

use crate::database::Database;
use crate::error::{EngineError, EngineResult};
use crate::ingestion::extract_pdf_text;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IngestedCell {
    pub cell_id: CellId,
    pub commit_seq: CommitSeq,
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
        let mut chunks = Vec::new();
        let mut current_chunk = String::new();
        for paragraph in text.split("\n\n") {
            let paragraph = paragraph.trim();
            if paragraph.is_empty() {
                continue;
            }
            if current_chunk.is_empty() {
                current_chunk = paragraph.to_owned();
            } else if current_chunk.len() + paragraph.len() < 1000 {
                current_chunk.push_str("\n\n");
                current_chunk.push_str(paragraph);
            } else {
                chunks.push(current_chunk);
                current_chunk = paragraph.to_owned();
            }
        }
        if !current_chunk.is_empty() {
            chunks.push(current_chunk);
        }

        if chunks.is_empty() {
            return Ok(Vec::new());
        }

        let mut ingested = Vec::new();
        for (index, chunk) in chunks.into_iter().enumerate() {
            let cell_id = offset_cell_id(first_cell_id, index)?;
            let commit_seq = self.put_knowledge_cell(
                cell_id,
                KnowledgeCell::new(
                    document_metadata(options.scope.clone(), options.source.clone()),
                    chunk,
                ),
            )?;
            ingested.push(IngestedCell {
                cell_id,
                commit_seq,
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
        flat_json_fields_serde(json)?
            .into_iter()
            .enumerate()
            .map(|(index, (key, value))| {
                let cell_id = offset_cell_id(first_cell_id, index)?;
                let body = format!("{key}: {value}");
                let commit_seq = self.put_knowledge_cell(
                    cell_id,
                    KnowledgeCell::new(
                        fact_metadata(options.scope.clone(), options.source.clone()),
                        body,
                    ),
                )?;
                Ok(IngestedCell {
                    cell_id,
                    commit_seq,
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
        let rows = csv_rows_serde(csv)?;
        let Some(headers) = rows.first() else {
            return Ok(Vec::new());
        };
        rows.iter()
            .skip(1)
            .enumerate()
            .map(|(index, row)| {
                let cell_id = offset_cell_id(first_cell_id, index)?;
                let body = headers
                    .iter()
                    .zip(row)
                    .map(|(header, value)| format!("{header}: {value}"))
                    .collect::<Vec<_>>()
                    .join("\n");
                let commit_seq = self.put_knowledge_cell(
                    cell_id,
                    KnowledgeCell::new(
                        document_metadata(options.scope.clone(), options.source.clone()),
                        body,
                    ),
                )?;
                Ok(IngestedCell {
                    cell_id,
                    commit_seq,
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
        let page = options
            .page
            .map(|value| format!("\npage={value}"))
            .unwrap_or_default();
        let body = format!("source_format=pdf{page}\n{extracted_text}");
        let commit_seq = self.put_knowledge_cell(
            cell_id,
            KnowledgeCell::new(document_metadata(options.scope, options.source), body),
        )?;
        Ok(IngestedCell {
            cell_id,
            commit_seq,
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

fn document_metadata(scope: String, source: String) -> KnowledgeCellMetadata {
    KnowledgeCellMetadata {
        scope,
        status: "ready".to_owned(),
        cell_type: KnowledgeCellType::DocumentBlock,
        source: Some(source),
        ..KnowledgeCellMetadata::default()
    }
}

fn fact_metadata(scope: String, source: String) -> KnowledgeCellMetadata {
    KnowledgeCellMetadata {
        scope,
        status: "ready".to_owned(),
        cell_type: KnowledgeCellType::Fact,
        source: Some(source),
        ..KnowledgeCellMetadata::default()
    }
}

fn offset_cell_id(first: CellId, offset: usize) -> EngineResult<CellId> {
    let offset = u64::try_from(offset)
        .map_err(|_| EngineError::StorageInvariant("ingestion batch is too large".to_owned()))?;
    first
        .0
        .checked_add(offset)
        .map(CellId)
        .ok_or_else(|| EngineError::StorageInvariant("cell id overflow".to_owned()))
}

fn flat_json_fields_serde(json: &str) -> EngineResult<Vec<(String, String)>> {
    let parsed: serde_json::Value = serde_json::from_str(json)
        .map_err(|e| EngineError::StorageInvariant(format!("invalid json: {e}")))?;
    let mut out = Vec::new();
    flatten_json_value(&parsed, "", &mut out);
    Ok(out)
}

fn flatten_json_value(value: &serde_json::Value, prefix: &str, out: &mut Vec<(String, String)>) {
    match value {
        serde_json::Value::Object(map) => {
            for (k, v) in map {
                let new_prefix = if prefix.is_empty() {
                    k.clone()
                } else {
                    format!("{}.{}", prefix, k)
                };
                flatten_json_value(v, &new_prefix, out);
            }
        }
        serde_json::Value::Array(arr) => {
            for (i, v) in arr.iter().enumerate() {
                flatten_json_value(v, &format!("{}.{}", prefix, i), out);
            }
        }
        serde_json::Value::String(s) => {
            out.push((prefix.to_owned(), s.clone()));
        }
        other => {
            out.push((prefix.to_owned(), other.to_string()));
        }
    }
}

fn csv_rows_serde(csv: &str) -> EngineResult<Vec<Vec<String>>> {
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(false)
        .from_reader(csv.as_bytes());
    let mut rows = Vec::new();
    for result in rdr.records() {
        let record =
            result.map_err(|e| EngineError::StorageInvariant(format!("csv error: {e}")))?;
        let mut row = Vec::new();
        for field in record.iter() {
            row.push(field.trim().to_owned());
        }
        rows.push(row);
    }
    Ok(rows)
}
