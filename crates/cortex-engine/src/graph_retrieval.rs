//! Multi-hop retrieval helpers over the lightweight knowledge graph.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use cortex_core::CellId;

use crate::database::Database;
use crate::graph::{GraphEdge, KnowledgeGraphIndex};

/// A cell reached through graph traversal.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GraphRetrievalHit {
    pub cell_id: CellId,
    pub matched_entity: String,
    pub depth: u32,
    pub proximity_score_q16: u16,
    pub explaining_edges: Vec<GraphEdge>,
}

impl Database {
    /// Retrieve visible cells related to an entity through graph edges.
    pub fn graph_retrieve_related(
        &self,
        seed_entity: &str,
        max_hops: u32,
    ) -> Vec<GraphRetrievalHit> {
        self.knowledge_graph_index()
            .retrieve_related_cells(seed_entity, max_hops)
    }
}

impl KnowledgeGraphIndex {
    /// Traverse relation edges breadth-first and return deterministic related cells.
    pub fn retrieve_related_cells(
        &self,
        seed_entity: &str,
        max_hops: u32,
    ) -> Vec<GraphRetrievalHit> {
        let seed_entity = seed_entity.trim();
        if seed_entity.is_empty() {
            return Vec::new();
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
        let mut queue = VecDeque::from([(seed_entity.to_owned(), 0_u32, Vec::<GraphEdge>::new())]);

        while let Some((entity_name, depth, path)) = queue.pop_front() {
            if depth >= max_hops {
                continue;
            }
            let next_depth = depth.saturating_add(1);
            for edge in self.neighbors(&entity_name) {
                let Some(next_entity) = other_endpoint(&edge, &entity_name) else {
                    continue;
                };
                let mut next_path = path.clone();
                next_path.push(edge.clone());

                insert_hit(
                    &mut hits,
                    GraphRetrievalHit {
                        cell_id: edge.relation_cell_id,
                        matched_entity: next_entity.clone(),
                        depth: next_depth,
                        proximity_score_q16: proximity_score_q16(next_depth),
                        explaining_edges: next_path.clone(),
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
                            explaining_edges: next_path.clone(),
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
        hits
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
