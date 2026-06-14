use std::collections::{BTreeMap, BTreeSet};

use cortex_core::CellId;

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
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct KnowledgeGraphIndex {
    pub(super) entities_by_name: BTreeMap<String, Vec<GraphEntity>>,
    pub(super) edges_by_entity: BTreeMap<String, Vec<GraphEdge>>,
    pub(super) edges_by_kind: BTreeMap<GraphEdgeKind, BTreeMap<CellId, GraphEdge>>,
    pub(super) source_support_edges_by_fact: BTreeMap<CellId, BTreeMap<CellId, GraphEdge>>,
    pub(super) cells_by_source: BTreeMap<String, BTreeSet<CellId>>,
}

/// A tool cell with its metadata.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToolCell {
    pub cell_id: CellId,
    pub name: Option<String>,
    pub description: String,
}
