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
        let commit_seq = self.put_knowledge_cell(
            cell_id,
            KnowledgeCell::new(document_metadata(options.scope, options.source), text),
        )?;
        Ok(IngestedCell {
            cell_id,
            commit_seq,
        })
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
        let rows = csv_rows(csv)?;
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

fn flat_json_fields(json: &str) -> EngineResult<Vec<(String, String)>> {
    let trimmed = json.trim();
    let body = trimmed
        .strip_prefix('{')
        .and_then(|value| value.strip_suffix('}'))
        .ok_or(EngineError::InvalidOperation)?;
    split_top_level(body, ',')
        .into_iter()
        .map(|pair| {
            let parts = split_top_level(pair, ':');
            if parts.len() != 2 {
                return Err(EngineError::InvalidOperation);
            }
            Ok((clean_json_value(parts[0]), clean_json_value(parts[1])))
        })
        .collect()
}

fn clean_json_value(value: &str) -> String {
    value.trim().trim_matches('"').replace("\\\"", "\"")
}

fn split_top_level(value: &str, delimiter: char) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut start = 0;
    let mut quoted = false;
    let mut escaped = false;
    for (index, character) in value.char_indices() {
        if escaped {
            escaped = false;
        } else if character == '\\' {
            escaped = true;
        } else if character == '"' {
            quoted = !quoted;
        } else if character == delimiter && !quoted {
            parts.push(&value[start..index]);
            start = index + character.len_utf8();
        }
    }
    parts.push(&value[start..]);
    parts
}

fn csv_rows(csv: &str) -> EngineResult<Vec<Vec<String>>> {
    csv.lines().map(csv_row).collect()
}

fn csv_row(line: &str) -> EngineResult<Vec<String>> {
    let mut values = Vec::new();
    let mut current = String::new();
    let mut quoted = false;
    let mut chars = line.chars().peekable();
    while let Some(character) = chars.next() {
        match character {
            '"' if quoted && chars.peek() == Some(&'"') => {
                current.push('"');
                chars.next();
            }
            '"' => quoted = !quoted,
            ',' if !quoted => {
                values.push(current.trim().to_owned());
                current.clear();
            }
            other => current.push(other),
        }
    }
    if quoted {
        return Err(EngineError::InvalidOperation);
    }
    values.push(current.trim().to_owned());
    Ok(values)
}
