use std::collections::BTreeMap;
use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};

use serde::Deserialize;
use serde_json::Value;

use super::parse_query_vector;

pub(crate) struct DocumentVectorLookup {
    offsets: BTreeMap<String, u64>,
    file: Option<File>,
}

#[derive(Deserialize)]
struct DocumentVectorIdRow {
    doc_id: String,
}

impl DocumentVectorLookup {
    pub(crate) fn empty() -> Self {
        Self {
            offsets: BTreeMap::new(),
            file: None,
        }
    }

    pub(crate) fn open(path: &std::path::Path) -> Result<Self, String> {
        let file = File::open(path).map_err(|error| {
            format!(
                "failed to open document vectors {}: {error}",
                path.display()
            )
        })?;
        let mut reader = BufReader::new(file);
        let mut offsets = BTreeMap::new();
        let mut line = String::new();
        loop {
            let offset = reader
                .stream_position()
                .map_err(|error| format!("failed to read {} offset: {error}", path.display()))?;
            line.clear();
            let bytes = reader
                .read_line(&mut line)
                .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
            if bytes == 0 {
                break;
            }
            let row: DocumentVectorIdRow =
                serde_json::from_str(line.trim_end()).map_err(|error| {
                    format!(
                        "failed to parse {} at byte {offset}: {error}",
                        path.display()
                    )
                })?;
            let doc_id = (!row.doc_id.trim().is_empty())
                .then_some(row.doc_id)
                .ok_or_else(|| format!("document vector row at byte {offset} missing doc_id"))?;
            offsets.insert(doc_id, offset);
        }
        Ok(Self {
            offsets,
            file: Some(reader.into_inner()),
        })
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.offsets.is_empty()
    }

    pub(crate) fn len(&self) -> usize {
        self.offsets.len()
    }

    pub(crate) fn get(&mut self, doc_id: &str) -> Result<Option<Vec<i16>>, String> {
        let Some(offset) = self.offsets.get(doc_id).copied() else {
            return Ok(None);
        };
        let file = self
            .file
            .as_mut()
            .ok_or_else(|| "document vector lookup has offsets without an open file".to_owned())?;
        file.seek(SeekFrom::Start(offset))
            .map_err(|error| format!("failed to seek document vector offset {offset}: {error}"))?;
        let mut reader = BufReader::new(file);
        let mut line = String::new();
        reader
            .read_line(&mut line)
            .map_err(|error| format!("failed to read document vector at byte {offset}: {error}"))?;
        let row: Value = serde_json::from_str(line.trim_end()).map_err(|error| {
            format!("failed to parse document vector at byte {offset}: {error}")
        })?;
        let vector = parse_query_vector(&row)
            .ok_or_else(|| format!("document vector for doc_id={doc_id} missing vector"))?;
        Ok(Some(vector))
    }
}
