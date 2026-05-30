use std::collections::{BTreeMap, BTreeSet};

mod graph;
mod index;
pub mod integrity;
mod maintenance;
mod metric;
mod search_impl;

pub use metric::{DistanceMetric, VectorCollectionConfig};

#[derive(Clone, Debug)]
pub struct HnswIndex {
    vectors: BTreeMap<u32, Vec<i16>>,
    links: BTreeMap<u32, BTreeSet<u32>>,
    upper_links: BTreeMap<u32, BTreeMap<u32, BTreeSet<u32>>>,
    deleted: BTreeSet<u32>,
    layer_count: usize,
    max_neighbors: usize,
    ef_search: usize,
    config: VectorCollectionConfig,
    pub rebuild_count: u64,
}
