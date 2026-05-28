//! Knowledge Graph traversal primitives.

use cortex_core::CellId;

use crate::database::Database;
use crate::query::metadata::CellMetadata;
use crate::typed_body::RelationBody;

/// A graph edge discovered through Relation cells.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GraphEdge {
    pub relation_cell_id: CellId,
    pub subject: String,
    pub predicate: String,
    pub object: String,
}

/// A tool cell with its metadata.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToolCell {
    pub cell_id: CellId,
    pub name: Option<String>,
    pub description: String,
}

impl Database {
    /// Find all Relation cells that connect to the given entity name
    /// (as either subject or object).
    pub fn graph_neighbors(&self, entity_name: &str) -> Vec<GraphEdge> {
        let mut edges = Vec::new();
        for version in self.snapshot_versions() {
            let metadata = CellMetadata::from_payload(&version.payload);
            if metadata.cell_type != "relation" {
                continue;
            }
            let relation = RelationBody::parse(&version.payload);
            let subject = relation.subject.unwrap_or_default();
            let object = relation.object.unwrap_or_default();
            if subject == entity_name || object == entity_name {
                edges.push(GraphEdge {
                    relation_cell_id: version.cell_id,
                    subject,
                    predicate: relation.predicate.unwrap_or_default(),
                    object,
                });
            }
        }
        edges
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
                    .map(|l| l.split_once('=').unwrap().1.to_owned());
                Some(ToolCell {
                    cell_id: version.cell_id,
                    name,
                    description: metadata.body_text,
                })
            })
            .collect()
    }
}
