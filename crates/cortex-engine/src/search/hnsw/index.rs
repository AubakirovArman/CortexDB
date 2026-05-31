use std::collections::{BTreeMap, BTreeSet};

use crate::error::EngineError;

use super::super::ranked;
use super::{HnswIndex, VectorCollectionConfig};

impl HnswIndex {
    pub fn new(max_neighbors: usize, ef_search: usize) -> Self {
        Self {
            vectors: BTreeMap::new(),
            links: BTreeMap::new(),
            upper_links: BTreeMap::new(),
            deleted: BTreeSet::new(),
            layer_count: 1,
            max_neighbors: max_neighbors.max(1),
            ef_search: ef_search.max(1),
            ef_construction: ef_search.max(max_neighbors).max(1),
            config: VectorCollectionConfig::default(),
            rebuild_count: 0,
        }
    }

    pub fn new_with_config(
        max_neighbors: usize,
        ef_search: usize,
        config: VectorCollectionConfig,
    ) -> Self {
        Self {
            config,
            ..Self::new(max_neighbors, ef_search)
        }
    }

    pub fn new_multilayer(max_neighbors: usize, ef_search: usize, layer_count: usize) -> Self {
        Self {
            layer_count: layer_count.max(1),
            ..Self::new(max_neighbors, ef_search)
        }
    }

    pub fn set_config(&mut self, config: VectorCollectionConfig) {
        self.config = config;
    }

    pub fn set_ef_construction(&mut self, ef_construction: usize) {
        self.ef_construction = ef_construction.max(self.max_neighbors).max(1);
    }

    pub fn add_vector(&mut self, cell_id: u32, vector: Vec<i16>) -> Result<(), EngineError> {
        if self.config.dimension > 0 && vector.len() != self.config.dimension {
            return Err(EngineError::VectorDimensionMismatch {
                expected: self.config.dimension,
                actual: vector.len(),
            });
        }
        self.deleted.remove(&cell_id);
        let mut neighbors = self.nearest_existing(&vector, self.ef_construction);
        neighbors.truncate(self.max_neighbors);
        for neighbor in &neighbors {
            self.links.entry(*neighbor).or_default().insert(cell_id);
        }
        self.links
            .insert(cell_id, neighbors.iter().copied().collect::<BTreeSet<_>>());
        let max_layer = self.max_layer_for_candidate(cell_id);
        for layer in 1..=max_layer {
            let layer_neighbors = self.nearest_existing_on_layer(layer, &vector);
            let layer_links = self.upper_links.entry(layer).or_default();
            for neighbor in &layer_neighbors {
                layer_links.entry(*neighbor).or_default().insert(cell_id);
            }
            layer_links.insert(cell_id, layer_neighbors.into_iter().collect());
        }
        self.vectors.insert(cell_id, vector);
        Ok(())
    }

    pub fn remove_vector(&mut self, cell_id: u32) -> bool {
        if self.vectors.contains_key(&cell_id) {
            self.deleted.insert(cell_id);
            self.links.remove(&cell_id);
            for neighbors in self.links.values_mut() {
                neighbors.remove(&cell_id);
            }
            for links in self.upper_links.values_mut() {
                links.remove(&cell_id);
                for neighbors in links.values_mut() {
                    neighbors.remove(&cell_id);
                }
            }
            true
        } else {
            false
        }
    }

    pub fn vector_count(&self) -> usize {
        self.vectors.len()
    }

    pub fn deleted_count(&self) -> usize {
        self.deleted.len()
    }

    pub fn layer_count(&self) -> usize {
        self.layer_count
    }

    pub fn ef_construction(&self) -> usize {
        self.ef_construction
    }

    fn nearest_existing(&self, vector: &[i16], limit: usize) -> Vec<u32> {
        let scores = self
            .vectors
            .iter()
            .filter(|(cell_id, _)| !self.deleted.contains(cell_id))
            .filter_map(|(cell_id, existing)| {
                self.config
                    .metric
                    .distance(vector, existing)
                    .map(|score| (*cell_id, score))
            })
            .collect();
        ranked(scores, limit)
            .into_iter()
            .map(|candidate| candidate.cell_id)
            .collect()
    }

    fn nearest_existing_on_layer(&self, layer: u32, vector: &[i16]) -> Vec<u32> {
        let Some(layer_links) = self.upper_links.get(&layer) else {
            return Vec::new();
        };
        let limit = self.max_neighbors.saturating_div(2).max(1);
        let scores = layer_links
            .keys()
            .filter(|cell_id| !self.deleted.contains(cell_id))
            .filter_map(|cell_id| {
                self.vectors
                    .get(cell_id)
                    .and_then(|existing| self.config.metric.distance(vector, existing))
                    .map(|score| (*cell_id, score))
            })
            .collect();
        ranked(scores, limit)
            .into_iter()
            .map(|candidate| candidate.cell_id)
            .collect()
    }

    fn max_layer_for_candidate(&self, cell_id: u32) -> u32 {
        if self.layer_count <= 1 {
            return 0;
        }
        let mut hash = deterministic_level_hash(cell_id);
        let mut layer = 0u32;
        while layer + 1 < self.layer_count as u32 && hash & 0b11 == 0 {
            layer += 1;
            hash >>= 2;
        }
        layer
    }
}

impl Default for HnswIndex {
    fn default() -> Self {
        Self::new(8, 32)
    }
}

fn deterministic_level_hash(cell_id: u32) -> u64 {
    let mut value = u64::from(cell_id).wrapping_add(0x9e37_79b9_7f4a_7c15);
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}
