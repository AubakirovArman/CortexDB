//! Multi-hop retrieval helpers over the lightweight knowledge graph.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use cortex_core::CellId;

use crate::database::Database;
use crate::graph::{GraphEdge, KnowledgeGraphIndex};

pub const DEFAULT_GRAPH_RETRIEVAL_VISIT_BUDGET: usize = 10_000;

/// A cell reached through graph traversal.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GraphRetrievalHit {
    pub cell_id: CellId,
    pub matched_entity: String,
    pub depth: u32,
    pub proximity_score_q16: u16,
    pub explaining_edges: Vec<GraphEdge>,
}

/// Deterministic graph traversal result plus bounded-visit accounting.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GraphRetrievalReport {
    pub hits: Vec<GraphRetrievalHit>,
    pub visited_entities: usize,
    pub visited_edges: usize,
    pub visit_budget: usize,
    pub budget_exceeded: bool,
}

impl Database {
    /// Retrieve visible cells related to an entity through graph edges.
    pub fn graph_retrieve_related(
        &self,
        seed_entity: &str,
        max_hops: u32,
    ) -> Vec<GraphRetrievalHit> {
        self.graph_retrieve_related_with_budget(
            seed_entity,
            max_hops,
            DEFAULT_GRAPH_RETRIEVAL_VISIT_BUDGET,
        )
        .hits
    }

    pub fn graph_retrieve_related_with_budget(
        &self,
        seed_entity: &str,
        max_hops: u32,
        visit_budget: usize,
    ) -> GraphRetrievalReport {
        self.derived_stores
            .graph_index_store
            .index_ref()
            .retrieve_related_cells_with_budget(seed_entity, max_hops, visit_budget)
    }
}

impl KnowledgeGraphIndex {
    /// Traverse relation edges breadth-first and return deterministic related cells.
    pub fn retrieve_related_cells(
        &self,
        seed_entity: &str,
        max_hops: u32,
    ) -> Vec<GraphRetrievalHit> {
        self.retrieve_related_cells_with_budget(
            seed_entity,
            max_hops,
            DEFAULT_GRAPH_RETRIEVAL_VISIT_BUDGET,
        )
        .hits
    }

    pub fn retrieve_related_cells_with_budget(
        &self,
        seed_entity: &str,
        max_hops: u32,
        visit_budget: usize,
    ) -> GraphRetrievalReport {
        let seed_entity = seed_entity.trim();
        let mut report = GraphRetrievalReport {
            hits: Vec::new(),
            visited_entities: 0,
            visited_edges: 0,
            visit_budget,
            budget_exceeded: false,
        };
        if seed_entity.is_empty() {
            return report;
        }

        let mut hits = BTreeMap::new();
        for entity in self.entities_named(seed_entity) {
            insert_hit(
                &mut hits,
                GraphRetrievalHit {
                    cell_id: entity.entity_cell_id,
                    matched_entity: seed_entity.to_owned(),
                    depth: 0,
                    proximity_score_q16: proximity_score_q16(0),
                    explaining_edges: Vec::new(),
                },
            );
        }

        let mut visited_entities = BTreeSet::from([seed_entity.to_owned()]);
        let mut queue = VecDeque::from([(seed_entity.to_owned(), 0_u32, Vec::<CellId>::new())]);

        'traversal: while let Some((entity_name, depth, path)) = queue.pop_front() {
            report.visited_entities = report.visited_entities.saturating_add(1);
            if depth >= max_hops {
                continue;
            }
            let next_depth = depth.saturating_add(1);
            for edge_id in self.neighbor_edge_ids(&entity_name) {
                if report.visited_edges >= visit_budget {
                    report.budget_exceeded = true;
                    break 'traversal;
                }
                report.visited_edges = report.visited_edges.saturating_add(1);
                let Some(edge) = self.edge_by_id(edge_id) else {
                    continue;
                };
                let Some(next_entity) = other_endpoint(edge, &entity_name) else {
                    continue;
                };
                let mut next_path = path.clone();
                next_path.push(edge.relation_cell_id);
                let explaining_edges = materialize_path(self, &next_path);

                insert_hit(
                    &mut hits,
                    GraphRetrievalHit {
                        cell_id: edge.relation_cell_id,
                        matched_entity: next_entity.clone(),
                        depth: next_depth,
                        proximity_score_q16: proximity_score_q16(next_depth),
                        explaining_edges: explaining_edges.clone(),
                    },
                );

                for entity in self.entities_named(&next_entity) {
                    insert_hit(
                        &mut hits,
                        GraphRetrievalHit {
                            cell_id: entity.entity_cell_id,
                            matched_entity: next_entity.clone(),
                            depth: next_depth,
                            proximity_score_q16: proximity_score_q16(next_depth),
                            explaining_edges: explaining_edges.clone(),
                        },
                    );
                }

                if visited_entities.insert(next_entity.clone()) {
                    queue.push_back((next_entity, next_depth, next_path));
                }
            }
        }

        let mut hits = hits.into_values().collect::<Vec<_>>();
        hits.sort_by_key(|hit| {
            (
                hit.depth,
                std::cmp::Reverse(hit.proximity_score_q16),
                hit.cell_id,
            )
        });
        report.hits = hits;
        report
    }
}

fn insert_hit(hits: &mut BTreeMap<CellId, GraphRetrievalHit>, hit: GraphRetrievalHit) {
    let replace = hits
        .get(&hit.cell_id)
        .map(|existing| {
            hit.depth < existing.depth
                || (hit.depth == existing.depth
                    && hit.explaining_edges.len() < existing.explaining_edges.len())
        })
        .unwrap_or(true);
    if replace {
        hits.insert(hit.cell_id, hit);
    }
}

fn other_endpoint(edge: &GraphEdge, entity_name: &str) -> Option<String> {
    if edge.subject == entity_name {
        non_empty_endpoint(&edge.object)
    } else if edge.object == entity_name {
        non_empty_endpoint(&edge.subject)
    } else {
        None
    }
}

fn non_empty_endpoint(value: &str) -> Option<String> {
    let value = value.trim();
    (!value.is_empty()).then(|| value.to_owned())
}

fn proximity_score_q16(depth: u32) -> u16 {
    if depth == 0 {
        return u16::MAX;
    }
    let divisor = depth.saturating_add(1);
    (u32::from(u16::MAX) / divisor).try_into().unwrap_or(0)
}

fn materialize_path(index: &KnowledgeGraphIndex, edge_ids: &[CellId]) -> Vec<GraphEdge> {
    edge_ids
        .iter()
        .filter_map(|edge_id| index.edge_by_id(*edge_id).cloned())
        .collect()
}
