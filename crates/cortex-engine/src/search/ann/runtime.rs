use cortex_storage::hnsw::HnswGraphIndex;

use super::super::hnsw::DistanceMetric;

#[derive(Clone, Copy, Debug)]
pub(super) struct HnswRuntimeConfig {
    pub(super) max_neighbors: usize,
    pub(super) ef_search: usize,
    pub(super) layer_count: usize,
    pub(super) upper_graph_edges: usize,
    pub(super) metric: DistanceMetric,
    pub(super) ef_construction: usize,
}

const ANN_DEFAULT_MAX_NEIGHBORS: usize = 8;
const ANN_DEFAULT_EF_SEARCH: usize = 64;
pub(super) fn hnsw_runtime_config(graph: &HnswGraphIndex) -> HnswRuntimeConfig {
    let max_neighbors = if graph.max_neighbors == 0 {
        ANN_DEFAULT_MAX_NEIGHBORS
    } else {
        graph.max_neighbors as usize
    };
    let ef_search = if graph.ef_search == 0 {
        ANN_DEFAULT_EF_SEARCH
    } else {
        graph.ef_search as usize
    };
    let layer_count = if graph.layer_count == 0 {
        graph
            .upper_layers
            .keys()
            .next_back()
            .and_then(|layer| layer.checked_add(1))
            .map(|layer| layer as usize)
            .unwrap_or(1)
    } else {
        graph.layer_count as usize
    };
    let upper_graph_edges = graph
        .upper_layers
        .values()
        .flat_map(|links| links.values())
        .map(|neighbors| neighbors.len())
        .sum();
    HnswRuntimeConfig {
        max_neighbors,
        ef_search,
        layer_count,
        upper_graph_edges,
        metric: metric_from_graph(graph),
        ef_construction: if graph.ef_construction == 0 {
            ef_search
        } else {
            graph.ef_construction as usize
        },
    }
}

pub(super) fn metric_from_graph(graph: &HnswGraphIndex) -> DistanceMetric {
    match graph.metric {
        1 => DistanceMetric::Cosine,
        2 => DistanceMetric::L2,
        _ => DistanceMetric::DotProduct,
    }
}
