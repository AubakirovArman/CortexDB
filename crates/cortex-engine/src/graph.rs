//! Knowledge Graph traversal primitives.

use std::collections::{BTreeMap, BTreeSet};

use cortex_core::CellId;

use crate::database::Database;
use crate::query::metadata::CellMetadata;
use crate::typed_body::{EntityBody, RelationBody};

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

    fn index_relation(&mut self, cell_id: CellId, payload: &[u8]) {
        let relation = RelationBody::parse(payload);
        let subject = relation.subject.unwrap_or_default();
        let object = relation.object.unwrap_or_default();
        if subject.trim().is_empty() || object.trim().is_empty() {
            return;
        }
        let edge = GraphEdge {
            relation_cell_id: cell_id,
            subject: subject.clone(),
            predicate: relation.predicate.unwrap_or_default(),
            object: object.clone(),
        };
        self.edges_by_entity
            .entry(subject)
            .or_default()
            .push(edge.clone());
        self.edges_by_entity.entry(object).or_default().push(edge);
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
}
