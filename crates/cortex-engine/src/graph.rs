//! Knowledge Graph traversal primitives.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use cortex_core::CellId;

use crate::database::Database;
use crate::error::{EngineError, EngineResult};
use crate::query::metadata::CellMetadata;
use crate::typed_body::{EntityBody, RelationBody};

const ACKG_MAGIC: &str = "CORTEXDB_ACKG_V1";
const ACKG_FILE_NAME: &str = "graph.ackg";
static ACKG_TEMP_COUNTER: AtomicU64 = AtomicU64::new(1);

/// A typed entity discovered through Entity cells.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GraphEntity {
    pub entity_cell_id: CellId,
    pub name: String,
    pub kind: Option<String>,
    pub source_id: Option<String>,
}

/// A graph edge discovered through Relation cells.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GraphEdge {
    pub relation_cell_id: CellId,
    pub subject: String,
    pub predicate: String,
    pub object: String,
    pub kind: GraphEdgeKind,
}

/// Stable graph edge categories used by higher-level retrieval and verification.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GraphEdgeKind {
    SourceSupportsFact,
    FactContradictsFact,
    Other,
}

/// Cells grouped by a source identifier.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GraphSourceRef {
    pub source_id: String,
    pub cell_ids: Vec<CellId>,
}

/// Snapshot graph indexes built from currently visible cells.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct KnowledgeGraphIndex {
    entities_by_name: BTreeMap<String, Vec<GraphEntity>>,
    edges_by_entity: BTreeMap<String, Vec<GraphEdge>>,
    cells_by_source: BTreeMap<String, BTreeSet<CellId>>,
}

/// A tool cell with its metadata.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToolCell {
    pub cell_id: CellId,
    pub name: Option<String>,
    pub description: String,
}

impl Database {
    /// Build a deterministic graph index from the current snapshot.
    pub fn knowledge_graph_index(&self) -> KnowledgeGraphIndex {
        KnowledgeGraphIndex::from_versions(self.snapshot_versions())
    }

    pub fn knowledge_graph_index_path(&self) -> PathBuf {
        self.root_path.join(ACKG_FILE_NAME)
    }

    pub fn persist_knowledge_graph_index(&self) -> EngineResult<KnowledgeGraphIndex> {
        let index = self.knowledge_graph_index();
        index.write_ackg(self.knowledge_graph_index_path())?;
        Ok(index)
    }

    pub fn read_persisted_knowledge_graph_index(
        &self,
    ) -> EngineResult<Option<KnowledgeGraphIndex>> {
        let path = self.knowledge_graph_index_path();
        if !path.exists() {
            return Ok(None);
        }
        KnowledgeGraphIndex::read_ackg(path).map(Some)
    }

    /// Find Entity cells by exact entity name.
    pub fn graph_entities(&self, entity_name: &str) -> Vec<GraphEntity> {
        self.knowledge_graph_index().entities_named(entity_name)
    }

    /// Find all Relation cells that connect to the given entity name
    /// (as either subject or object).
    pub fn graph_neighbors(&self, entity_name: &str) -> Vec<GraphEdge> {
        self.knowledge_graph_index().neighbors(entity_name)
    }

    /// Find visible cells associated with a source identifier.
    pub fn graph_cells_for_source(&self, source_id: &str) -> Vec<CellId> {
        self.knowledge_graph_index().cells_for_source(source_id)
    }

    /// Find visible relation cells that connect a source to a fact.
    pub fn graph_source_supports_fact_edges(&self) -> Vec<GraphEdge> {
        self.knowledge_graph_index().source_supports_fact_edges()
    }

    /// Find visible relation cells that mark one fact as contradicting another.
    pub fn graph_fact_contradicts_fact_edges(&self) -> Vec<GraphEdge> {
        self.knowledge_graph_index().fact_contradicts_fact_edges()
    }

    /// Find all Tool cells in the database.
    pub fn tool_cells(&self) -> Vec<ToolCell> {
        self.snapshot_versions()
            .into_iter()
            .filter_map(|version| {
                let metadata = CellMetadata::from_payload(&version.payload);
                if metadata.cell_type != "tool" {
                    return None;
                }
                let body = String::from_utf8_lossy(&version.payload);
                let name = body
                    .lines()
                    .find(|l| l.starts_with("name="))
                    .and_then(|line| line.strip_prefix("name="))
                    .map(ToOwned::to_owned);
                Some(ToolCell {
                    cell_id: version.cell_id,
                    name,
                    description: metadata.body_text,
                })
            })
            .collect()
    }
}

impl KnowledgeGraphIndex {
    pub fn from_versions(versions: Vec<cortex_core::memtable::CellVersion>) -> Self {
        let mut index = Self::default();
        for version in versions {
            let metadata = CellMetadata::from_payload(&version.payload);
            index.index_source_ref(version.cell_id, &metadata);
            match metadata.cell_type.as_str() {
                "entity" => index.index_entity(version.cell_id, &version.payload, &metadata),
                "relation" => index.index_relation(version.cell_id, &version.payload),
                _ => {}
            }
        }
        index.sort();
        index
    }

    pub fn entities_named(&self, entity_name: &str) -> Vec<GraphEntity> {
        self.entities_by_name
            .get(entity_name)
            .cloned()
            .unwrap_or_default()
    }

    pub fn neighbors(&self, entity_name: &str) -> Vec<GraphEdge> {
        self.edges_by_entity
            .get(entity_name)
            .cloned()
            .unwrap_or_default()
    }

    pub fn cells_for_source(&self, source_id: &str) -> Vec<CellId> {
        self.cells_by_source
            .get(source_id)
            .map(|cells| cells.iter().copied().collect())
            .unwrap_or_default()
    }

    pub fn source_refs(&self) -> Vec<GraphSourceRef> {
        self.cells_by_source
            .iter()
            .map(|(source_id, cells)| GraphSourceRef {
                source_id: source_id.clone(),
                cell_ids: cells.iter().copied().collect(),
            })
            .collect()
    }

    pub fn source_supports_fact_edges(&self) -> Vec<GraphEdge> {
        self.edges_by_kind(GraphEdgeKind::SourceSupportsFact)
    }

    pub fn fact_contradicts_fact_edges(&self) -> Vec<GraphEdge> {
        self.edges_by_kind(GraphEdgeKind::FactContradictsFact)
    }

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

    fn index_entity(&mut self, cell_id: CellId, payload: &[u8], metadata: &CellMetadata) {
        let entity = EntityBody::parse(payload);
        let Some(name) = entity.name.filter(|name| !name.trim().is_empty()) else {
            return;
        };
        self.entities_by_name
            .entry(name.clone())
            .or_default()
            .push(GraphEntity {
                entity_cell_id: cell_id,
                name,
                kind: entity.kind,
                source_id: metadata
                    .source_ref
                    .as_ref()
                    .map(|source_ref| source_ref.source_id.clone()),
            });
    }

    fn push_entity(&mut self, entity: GraphEntity) {
        self.entities_by_name
            .entry(entity.name.clone())
            .or_default()
            .push(entity);
    }

    fn index_relation(&mut self, cell_id: CellId, payload: &[u8]) {
        let relation = RelationBody::parse(payload);
        let subject = relation.subject.unwrap_or_default();
        let object = relation.object.unwrap_or_default();
        if subject.trim().is_empty() || object.trim().is_empty() {
            return;
        }
        let predicate = relation.predicate.unwrap_or_default();
        let edge = GraphEdge {
            relation_cell_id: cell_id,
            subject: subject.clone(),
            predicate: predicate.clone(),
            object: object.clone(),
            kind: GraphEdgeKind::from_predicate(&predicate),
        };
        self.push_edge(edge);
    }

    fn push_edge(&mut self, edge: GraphEdge) {
        self.edges_by_entity
            .entry(edge.subject.clone())
            .or_default()
            .push(edge.clone());
        self.edges_by_entity
            .entry(edge.object.clone())
            .or_default()
            .push(edge);
    }

    fn index_source_ref(&mut self, cell_id: CellId, metadata: &CellMetadata) {
        let Some(source_ref) = &metadata.source_ref else {
            return;
        };
        self.cells_by_source
            .entry(source_ref.source_id.clone())
            .or_default()
            .insert(cell_id);
    }

    fn sort(&mut self) {
        for entities in self.entities_by_name.values_mut() {
            entities.sort_by_key(|entity| entity.entity_cell_id);
        }
        for edges in self.edges_by_entity.values_mut() {
            edges.sort_by_key(|edge| {
                (
                    edge.relation_cell_id,
                    edge.subject.clone(),
                    edge.predicate.clone(),
                    edge.object.clone(),
                )
            });
        }
    }

    fn edges_by_kind(&self, kind: GraphEdgeKind) -> Vec<GraphEdge> {
        let mut edges = self
            .edges_by_entity
            .values()
            .flat_map(|edges| edges.iter())
            .filter(|edge| edge.kind == kind)
            .cloned()
            .collect::<Vec<_>>();
        edges.sort_by_key(|edge| {
            (
                edge.relation_cell_id,
                edge.subject.clone(),
                edge.predicate.clone(),
                edge.object.clone(),
            )
        });
        edges.dedup_by_key(|edge| edge.relation_cell_id);
        edges
    }

    fn all_entities(&self) -> Vec<GraphEntity> {
        let mut entities = self
            .entities_by_name
            .values()
            .flat_map(|entities| entities.iter())
            .cloned()
            .collect::<Vec<_>>();
        entities.sort_by_key(|entity| {
            (
                entity.entity_cell_id,
                entity.name.clone(),
                entity.kind.clone(),
                entity.source_id.clone(),
            )
        });
        entities
    }

    fn all_edges(&self) -> Vec<GraphEdge> {
        let mut edges = self
            .edges_by_entity
            .values()
            .flat_map(|edges| edges.iter())
            .cloned()
            .collect::<Vec<_>>();
        edges.sort_by_key(|edge| {
            (
                edge.relation_cell_id,
                edge.subject.clone(),
                edge.predicate.clone(),
                edge.object.clone(),
            )
        });
        edges.dedup_by_key(|edge| edge.relation_cell_id);
        edges
    }
}

impl GraphEdgeKind {
    pub fn from_predicate(predicate: &str) -> Self {
        match normalize_predicate(predicate).as_str() {
            "source_supports_fact" | "supports_fact" | "source_supports" => {
                Self::SourceSupportsFact
            }
            "fact_contradicts_fact" | "contradicts_fact" | "contradicts" => {
                Self::FactContradictsFact
            }
            _ => Self::Other,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::SourceSupportsFact => "source_supports_fact",
            Self::FactContradictsFact => "fact_contradicts_fact",
            Self::Other => "other",
        }
    }

    fn from_ackg_kind(value: &str) -> Self {
        match value {
            "source_supports_fact" => Self::SourceSupportsFact,
            "fact_contradicts_fact" => Self::FactContradictsFact,
            _ => Self::Other,
        }
    }
}

fn normalize_predicate(predicate: &str) -> String {
    predicate.trim().to_lowercase().replace([' ', '-'], "_")
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn persisted_knowledge_graph_survives_restart_without_rescan() {
        let dir = tempfile::tempdir().unwrap();
        {
            let mut db = Database::open(dir.path()).unwrap();
            db.put_cell(
                CellId(1),
                b"scope=default\nstatus=ready\ntype=entity\nsource=crm\n\nname=Apollo\nkind=project"
                    .to_vec(),
            )
            .unwrap();
            db.put_cell(
                CellId(2),
                b"scope=default\nstatus=ready\ntype=relation\n\nsubject=source-1\npredicate=source_supports_fact\nobject=fact-1"
                    .to_vec(),
            )
            .unwrap();
            let index = db.persist_knowledge_graph_index().unwrap();
            assert_eq!(index.entities_named("Apollo").len(), 1);
            assert_eq!(index.source_supports_fact_edges().len(), 1);
            assert!(db.knowledge_graph_index_path().exists());
        }

        let db = Database::open(dir.path()).unwrap();
        let persisted = db.read_persisted_knowledge_graph_index().unwrap().unwrap();

        assert_eq!(
            persisted.entities_named("Apollo")[0].kind.as_deref(),
            Some("project")
        );
        assert_eq!(
            persisted.source_supports_fact_edges()[0].predicate,
            "source_supports_fact"
        );
        assert_eq!(persisted.cells_for_source("crm"), vec![CellId(1)]);
    }

    #[test]
    fn ackg_roundtrip_preserves_escaped_fields() {
        let mut index = KnowledgeGraphIndex::default();
        index.push_entity(GraphEntity {
            entity_cell_id: CellId(7),
            name: "Apollo\tProject".to_owned(),
            kind: Some("project\nteam".to_owned()),
            source_id: Some("crm".to_owned()),
        });
        index.push_edge(GraphEdge {
            relation_cell_id: CellId(8),
            subject: "Apollo\tProject".to_owned(),
            predicate: "related_to".to_owned(),
            object: "Budget\nPlan".to_owned(),
            kind: GraphEdgeKind::Other,
        });
        let decoded = KnowledgeGraphIndex::from_ackg_bytes(&index.to_ackg_bytes()).unwrap();

        assert_eq!(decoded.entities_named("Apollo\tProject").len(), 1);
        assert_eq!(decoded.neighbors("Budget\nPlan").len(), 1);
    }
}
