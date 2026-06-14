use std::collections::BTreeSet;

use cortex_core::CellId;

use super::types::{GraphEdge, GraphEntity, GraphNodeId, KnowledgeGraphIndex};

pub(super) fn remove_empty_values<K: Ord, V>(
    values: &mut std::collections::BTreeMap<K, V>,
    mut should_remove: impl FnMut(&mut V) -> bool,
) {
    values.retain(|_, value| !should_remove(value));
}

pub(super) fn fact_cell_endpoint(edge: &GraphEdge) -> Option<CellId> {
    cell_endpoint(&edge.object).or_else(|| cell_endpoint(&edge.subject))
}

pub(super) fn cell_endpoint(value: &str) -> Option<CellId> {
    value
        .trim()
        .strip_prefix("cell:")
        .and_then(|id| id.parse::<u64>().ok())
        .map(CellId)
}

pub(super) fn sort_edges(edges: &mut [GraphEdge]) {
    edges.sort_by_key(|edge| {
        (
            edge.relation_cell_id,
            edge.subject.clone(),
            edge.predicate.clone(),
            edge.object.clone(),
        )
    });
}

pub(super) fn insert_cell_id_sorted(values: &mut Vec<CellId>, cell_id: CellId) {
    match values.binary_search(&cell_id) {
        Ok(_) => {}
        Err(index) => values.insert(index, cell_id),
    }
}

pub(super) fn insert_entity_sorted(entities: &mut Vec<GraphEntity>, entity: GraphEntity) {
    let index = entities
        .binary_search_by_key(&entity.entity_cell_id, |candidate| candidate.entity_cell_id)
        .unwrap_or_else(|index| index);
    entities.insert(index, entity);
}

pub(super) fn canonicalize_index(index: &mut KnowledgeGraphIndex) {
    for entities in index.entities_by_name.values_mut() {
        entities.sort_by_key(|entity| entity.entity_cell_id);
    }
    let mut entity_names = index
        .entities_by_name
        .keys()
        .cloned()
        .collect::<BTreeSet<_>>();
    for edge in index.edges_by_id.values() {
        entity_names.insert(edge.subject.clone());
        entity_names.insert(edge.object.clone());
    }
    index.entity_name_to_node.clear();
    index.entity_names_by_node.clear();
    index.edge_ids_by_entity.clear();
    for entity_name in entity_names {
        let node_id = index.entity_names_by_node.len() as GraphNodeId;
        index.entity_names_by_node.push(entity_name.clone());
        index.entity_name_to_node.insert(entity_name, node_id);
    }
    let edges = index
        .edges_by_id
        .values()
        .map(|edge| {
            (
                edge.subject.clone(),
                edge.object.clone(),
                edge.relation_cell_id,
            )
        })
        .collect::<Vec<_>>();
    for (subject, object, edge_id) in edges {
        push_edge_id_for_entity(index, &subject, edge_id);
        if object != subject {
            push_edge_id_for_entity(index, &object, edge_id);
        }
    }
}

fn push_edge_id_for_entity(index: &mut KnowledgeGraphIndex, entity_name: &str, edge_id: CellId) {
    let Some(node_id) = index.entity_name_to_node.get(entity_name) else {
        return;
    };
    insert_cell_id_sorted(
        index.edge_ids_by_entity.entry(*node_id).or_default(),
        edge_id,
    );
}
