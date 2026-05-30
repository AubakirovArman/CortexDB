use std::collections::BTreeMap;

use cortex_storage::hnsw::HnswGraphIndex;

use super::{DistanceMetric, HnswIndex, VectorCollectionConfig};

impl HnswIndex {
    pub fn graph_index(&self) -> HnswGraphIndex {
        HnswGraphIndex {
            links: self.links.clone(),
            dimension: self.config.dimension as u32,
            metric: self.config.metric as u8,
            upper_layers: self.upper_links.clone(),
            max_neighbors: usize_to_u32(self.max_neighbors),
            ef_search: usize_to_u32(self.ef_search),
            layer_count: usize_to_u32(self.layer_count),
        }
    }

    pub fn from_graph(
        vectors: BTreeMap<u32, Vec<i16>>,
        graph: HnswGraphIndex,
        max_neighbors: usize,
        ef_search: usize,
    ) -> Self {
        let dimension = if graph.dimension > 0 {
            graph.dimension as usize
        } else {
            vectors.values().next().map(|v| v.len()).unwrap_or(0)
        };
        let metric = match graph.metric {
            1 => DistanceMetric::Cosine,
            2 => DistanceMetric::L2,
            _ => DistanceMetric::DotProduct,
        };
        let graph_max_neighbors = graph.max_neighbors;
        let graph_ef_search = graph.ef_search;
        let graph_layer_count = graph.layer_count;
        let layer_count = if graph_layer_count > 0 {
            graph_layer_count as usize
        } else {
            graph
                .upper_layers
                .keys()
                .next_back()
                .map(|layer| (*layer as usize) + 1)
                .unwrap_or(1)
        };
        Self {
            vectors,
            links: graph.links,
            upper_links: graph.upper_layers,
            deleted: Default::default(),
            layer_count,
            max_neighbors: graph_value_or(graph_max_neighbors, max_neighbors),
            ef_search: graph_value_or(graph_ef_search, ef_search),
            config: VectorCollectionConfig { dimension, metric },
            rebuild_count: 0,
        }
    }
}

fn graph_value_or(value: u32, fallback: usize) -> usize {
    if value == 0 {
        fallback.max(1)
    } else {
        value as usize
    }
}

fn usize_to_u32(value: usize) -> u32 {
    if value > u32::MAX as usize {
        u32::MAX
    } else {
        value as u32
    }
}
