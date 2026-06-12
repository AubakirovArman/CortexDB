use cortex_core::CellId;

use crate::query::metadata::CellMetadata;
use crate::typed_body::{EntityBody, RelationBody};

use super::types::{GraphEdge, GraphEdgeKind, GraphEntity, GraphSourceRef, KnowledgeGraphIndex};

impl KnowledgeGraphIndex {
    pub fn from_versions(versions: Vec<cortex_core::memtable::CellVersion>) -> Self {
        let mut index = Self::default();
        for version in versions {
            let metadata = CellMetadata::from_version(&version);
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

    pub(super) fn push_entity(&mut self, entity: GraphEntity) {
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

    pub(super) fn push_edge(&mut self, edge: GraphEdge) {
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

    pub(super) fn sort(&mut self) {
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
        sort_edges(&mut edges);
        edges.dedup_by_key(|edge| edge.relation_cell_id);
        edges
    }

    pub(super) fn all_entities(&self) -> Vec<GraphEntity> {
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

    pub(super) fn all_edges(&self) -> Vec<GraphEdge> {
        let mut edges = self
            .edges_by_entity
            .values()
            .flat_map(|edges| edges.iter())
            .cloned()
            .collect::<Vec<_>>();
        sort_edges(&mut edges);
        edges.dedup_by_key(|edge| edge.relation_cell_id);
        edges
    }
}

fn sort_edges(edges: &mut [GraphEdge]) {
    edges.sort_by_key(|edge| {
        (
            edge.relation_cell_id,
            edge.subject.clone(),
            edge.predicate.clone(),
            edge.object.clone(),
        )
    });
}
