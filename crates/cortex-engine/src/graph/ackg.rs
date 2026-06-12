use std::fs;
use std::io::Write;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

use cortex_core::CellId;

use crate::error::{EngineError, EngineResult};

use super::types::{GraphEdge, GraphEdgeKind, GraphEntity, KnowledgeGraphIndex};

const ACKG_MAGIC: &str = "CORTEXDB_ACKG_V1";
pub(super) const ACKG_FILE_NAME: &str = "graph.ackg";
static ACKG_TEMP_COUNTER: AtomicU64 = AtomicU64::new(1);

impl KnowledgeGraphIndex {
    pub fn to_ackg_bytes(&self) -> Vec<u8> {
        let mut out = String::new();
        out.push_str(ACKG_MAGIC);
        out.push('\n');
        for entity in self.all_entities() {
            push_ackg_line(
                &mut out,
                &[
                    "entity",
                    &entity.entity_cell_id.0.to_string(),
                    &encode_field(&entity.name),
                    &encode_optional_field(entity.kind.as_deref()),
                    &encode_optional_field(entity.source_id.as_deref()),
                ],
            );
        }
        for edge in self.all_edges() {
            push_ackg_line(
                &mut out,
                &[
                    "edge",
                    &edge.relation_cell_id.0.to_string(),
                    &encode_field(&edge.subject),
                    &encode_field(&edge.predicate),
                    &encode_field(&edge.object),
                    edge.kind.as_str(),
                ],
            );
        }
        for source_ref in self.source_refs() {
            let cell_ids = source_ref
                .cell_ids
                .iter()
                .map(|cell_id| cell_id.0.to_string())
                .collect::<Vec<_>>()
                .join(",");
            push_ackg_line(
                &mut out,
                &["source", &encode_field(&source_ref.source_id), &cell_ids],
            );
        }
        out.into_bytes()
    }

    pub fn from_ackg_bytes(bytes: &[u8]) -> EngineResult<Self> {
        let text = std::str::from_utf8(bytes)
            .map_err(|_| EngineError::StorageInvariant("invalid ACKG utf8".to_owned()))?;
        let mut lines = text.lines();
        if lines.next() != Some(ACKG_MAGIC) {
            return Err(EngineError::StorageInvariant(
                "invalid ACKG magic".to_owned(),
            ));
        }
        let mut index = Self::default();
        for (line_index, line) in lines.enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            let fields = line.split('\t').collect::<Vec<_>>();
            match fields.as_slice() {
                ["entity", cell_id, name, kind, source_id] => {
                    index.push_entity(GraphEntity {
                        entity_cell_id: parse_cell_id(cell_id, line_index)?,
                        name: decode_field(name)?,
                        kind: decode_optional_field(kind)?,
                        source_id: decode_optional_field(source_id)?,
                    });
                }
                ["edge", cell_id, subject, predicate, object, kind] => {
                    index.push_edge(GraphEdge {
                        relation_cell_id: parse_cell_id(cell_id, line_index)?,
                        subject: decode_field(subject)?,
                        predicate: decode_field(predicate)?,
                        object: decode_field(object)?,
                        kind: GraphEdgeKind::from_ackg_kind(kind),
                    });
                }
                ["source", source_id, cell_ids] => {
                    let source_id = decode_field(source_id)?;
                    for cell_id in parse_cell_ids(cell_ids, line_index)? {
                        index
                            .cells_by_source
                            .entry(source_id.clone())
                            .or_default()
                            .insert(cell_id);
                    }
                }
                _ => {
                    return Err(EngineError::StorageInvariant(format!(
                        "invalid ACKG line {}",
                        line_index + 2
                    )));
                }
            }
        }
        index.sort();
        Ok(index)
    }

    pub fn write_ackg(&self, path: impl AsRef<Path>) -> EngineResult<()> {
        write_atomic(path.as_ref(), &self.to_ackg_bytes())
    }

    pub fn read_ackg(path: impl AsRef<Path>) -> EngineResult<Self> {
        let bytes = fs::read(path)?;
        Self::from_ackg_bytes(&bytes)
    }
}

fn push_ackg_line(out: &mut String, fields: &[&str]) {
    out.push_str(&fields.join("\t"));
    out.push('\n');
}

fn encode_optional_field(value: Option<&str>) -> String {
    value.map(encode_field).unwrap_or_else(|| "-".to_owned())
}

fn decode_optional_field(value: &str) -> EngineResult<Option<String>> {
    if value == "-" {
        Ok(None)
    } else {
        decode_field(value).map(Some)
    }
}

fn encode_field(value: &str) -> String {
    value
        .as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn decode_field(value: &str) -> EngineResult<String> {
    if !value.len().is_multiple_of(2) {
        return Err(EngineError::StorageInvariant(
            "invalid ACKG hex field".to_owned(),
        ));
    }
    let mut bytes = Vec::with_capacity(value.len() / 2);
    for chunk in value.as_bytes().chunks_exact(2) {
        let part = std::str::from_utf8(chunk)
            .map_err(|_| EngineError::StorageInvariant("invalid ACKG hex".to_owned()))?;
        let byte = u8::from_str_radix(part, 16)
            .map_err(|_| EngineError::StorageInvariant("invalid ACKG hex".to_owned()))?;
        bytes.push(byte);
    }
    String::from_utf8(bytes)
        .map_err(|_| EngineError::StorageInvariant("invalid ACKG string".to_owned()))
}

fn parse_cell_id(value: &str, line_index: usize) -> EngineResult<CellId> {
    value.parse::<u64>().map(CellId).map_err(|_| {
        EngineError::StorageInvariant(format!("invalid ACKG cell id at line {}", line_index + 2))
    })
}

fn parse_cell_ids(value: &str, line_index: usize) -> EngineResult<Vec<CellId>> {
    if value.is_empty() {
        return Ok(Vec::new());
    }
    value
        .split(',')
        .map(|part| parse_cell_id(part, line_index))
        .collect()
}

fn write_atomic(path: &Path, bytes: &[u8]) -> EngineResult<()> {
    let counter = ACKG_TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or(ACKG_FILE_NAME);
    let tmp = path.with_file_name(format!(
        "{file_name}.tmp.{}.{}",
        std::process::id(),
        counter
    ));
    {
        let mut file = fs::File::create(&tmp)?;
        file.write_all(bytes)?;
        file.sync_all()?;
    }
    fs::rename(&tmp, path)?;
    if let Some(parent) = path.parent() {
        fs::OpenOptions::new().read(true).open(parent)?.sync_all()?;
    }
    Ok(())
}
