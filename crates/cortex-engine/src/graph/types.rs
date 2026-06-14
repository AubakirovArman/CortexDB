use std::collections::{BTreeMap, BTreeSet};

use cortex_core::CellId;

pub(super) type GraphNodeId = u32;

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
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
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
#[derive(Clone, Debug, Default)]
pub struct KnowledgeGraphIndex {
    pub(super) entities_by_name: BTreeMap<String, Vec<GraphEntity>>,
    pub(super) entity_name_to_node: BTreeMap<String, GraphNodeId>,
    pub(super) entity_names_by_node: Vec<String>,
    pub(super) edge_ids_by_entity: BTreeMap<GraphNodeId, Vec<CellId>>,
    pub(super) edges_by_id: BTreeMap<CellId, GraphEdge>,
    pub(super) edges_by_kind: BTreeMap<GraphEdgeKind, BTreeMap<CellId, GraphEdge>>,
    pub(super) source_support_edges_by_fact: BTreeMap<CellId, BTreeMap<CellId, GraphEdge>>,
    pub(super) cells_by_source: BTreeMap<String, BTreeSet<CellId>>,
}

impl PartialEq for KnowledgeGraphIndex {
    fn eq(&self, other: &Self) -> bool {
        self.entities_by_name == other.entities_by_name
            && self.adjacency_by_name() == other.adjacency_by_name()
            && self.edges_by_id == other.edges_by_id
            && self.edges_by_kind == other.edges_by_kind
            && self.source_support_edges_by_fact == other.source_support_edges_by_fact
            && self.cells_by_source == other.cells_by_source
    }
}

impl Eq for KnowledgeGraphIndex {}

impl KnowledgeGraphIndex {
    fn adjacency_by_name(&self) -> BTreeMap<String, Vec<CellId>> {
        self.entity_name_to_node
            .iter()
            .filter_map(|(entity_name, node_id)| {
                let edge_ids = self.edge_ids_by_entity.get(node_id)?;
                (!edge_ids.is_empty()).then(|| (entity_name.clone(), edge_ids.clone()))
            })
            .collect()
    }
}

/// A tool cell with its metadata.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToolCell {
    pub cell_id: CellId,
    pub name: Option<String>,
    pub description: String,
}
