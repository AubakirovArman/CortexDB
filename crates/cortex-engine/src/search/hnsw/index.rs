use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet};

use crate::error::EngineError;

use super::super::ranked;
use super::search_impl::{
    first_live_node_in_map, pop_best_frontier, pop_worst_score, push_candidate,
};
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

    /// Inserts a vector via standard HNSW graph descent — deterministic entry
    /// point, greedy upper-layer descent, an `ef_construction` beam per layer,
    /// diversity neighbor selection, and bounded-degree bidirectional linking —
    /// replacing the O(N) full-scan `nearest_existing` (kept as a test oracle).
    /// No RNG or wall clock: level assignment is the deterministic hash and every
    /// tie breaks on higher score then smaller cell id, so a rebuild is
    /// byte-identical.
    pub fn add_vector(&mut self, cell_id: u32, vector: Vec<i16>) -> Result<(), EngineError> {
        if self.config.dimension > 0 && vector.len() != self.config.dimension {
            return Err(EngineError::VectorDimensionMismatch {
                expected: self.config.dimension,
                actual: vector.len(),
            });
        }
        self.deleted.remove(&cell_id);
        let new_max_layer = self.max_layer_for_candidate(cell_id);

        let Some(mut entry) = self.build_entry_point() else {
            self.links.entry(cell_id).or_default();
            for layer in 1..=new_max_layer {
                self.upper_links
                    .entry(layer)
                    .or_default()
                    .entry(cell_id)
                    .or_default();
            }
            self.vectors.insert(cell_id, vector);
            return Ok(());
        };

        let top_layer = self.top_populated_layer();
        let mut layer = top_layer;
        while layer > new_max_layer {
            entry = self.descend_build_layer(layer, &vector, entry);
            layer -= 1;
        }

        let mut entry_points = vec![entry];
        let start_layer = new_max_layer.min(top_layer);
        for layer in (0..=start_layer).rev() {
            let candidates = self.search_build_layer(layer, &vector, &entry_points);
            let degree = self.layer_degree(layer);
            // New node keeps its diverse links plus a backfill of the nearest
            // pruned candidates so its out-degree stays healthy even in low
            // dimensions (keep_pruned = true).
            let selected = self.select_neighbors_heuristic(&vector, &candidates, degree, true);
            self.connect_and_prune(cell_id, layer, &selected);
            entry_points = if candidates.is_empty() {
                vec![entry]
            } else {
                candidates
            };
        }

        for layer in (top_layer + 1)..=new_max_layer {
            self.upper_links
                .entry(layer)
                .or_default()
                .entry(cell_id)
                .or_default();
        }
        self.links.entry(cell_id).or_default();

        self.vectors.insert(cell_id, vector);
        Ok(())
    }

    fn build_entry_point(&self) -> Option<u32> {
        self.upper_links
            .iter()
            .rev()
            .find_map(|(_, layer_links)| first_live_node_in_map(layer_links, &self.deleted))
            .or_else(|| first_live_node_in_map(&self.links, &self.deleted))
            .or_else(|| {
                self.vectors
                    .keys()
                    .find(|cell_id| !self.deleted.contains(cell_id))
                    .copied()
            })
    }

    fn top_populated_layer(&self) -> u32 {
        self.upper_links
            .iter()
            .rev()
            .find(|(_, links)| links.keys().any(|id| !self.deleted.contains(id)))
            .map(|(layer, _)| *layer)
            .unwrap_or(0)
    }

    /// Max out-degree per layer: layer 0 uses M0 = 2·M (hnswlib convention) so
    /// diverse long-range edges and nearer backfilled edges coexist; upper
    /// layers use M.
    fn layer_degree(&self, layer: u32) -> usize {
        if layer == 0 {
            self.max_neighbors.saturating_mul(2).max(1)
        } else {
            self.max_neighbors.max(1)
        }
    }

    fn descend_build_layer(&self, layer: u32, query: &[i16], entry: u32) -> u32 {
        let Some(layer_links) = self.upper_links.get(&layer) else {
            return entry;
        };
        let mut current = entry;
        loop {
            let Some(current_vector) = self.vectors.get(&current) else {
                return current;
            };
            let Some(mut best_score) = self.config.metric.distance(query, current_vector) else {
                return current;
            };
            let mut best = current;
            if let Some(neighbors) = layer_links.get(&current) {
                for neighbor in neighbors {
                    if self.deleted.contains(neighbor) {
                        continue;
                    }
                    let Some(neighbor_vector) = self.vectors.get(neighbor) else {
                        continue;
                    };
                    let Some(neighbor_score) = self.config.metric.distance(query, neighbor_vector)
                    else {
                        continue;
                    };
                    if neighbor_score > best_score
                        || (neighbor_score == best_score && *neighbor < best)
                    {
                        best = *neighbor;
                        best_score = neighbor_score;
                    }
                }
            }
            if best == current {
                return current;
            }
            current = best;
        }
    }

    fn search_build_layer(&self, layer: u32, query: &[i16], entry_points: &[u32]) -> Vec<u32> {
        let adjacency = if layer == 0 {
            &self.links
        } else {
            match self.upper_links.get(&layer) {
                Some(links) => links,
                None => return Vec::new(),
            }
        };
        let ef = self.ef_construction.max(1);
        let mut visited = BTreeSet::new();
        let mut frontier: BTreeSet<(Reverse<u64>, u32)> = BTreeSet::new();
        let mut top_scores: BTreeSet<(u64, u32)> = BTreeSet::new();
        let mut scores: BTreeMap<u32, u64> = BTreeMap::new();

        for entry in entry_points {
            if let Some(vector) = self.vectors.get(entry) {
                if let Some(score) = self.config.metric.distance(query, vector) {
                    frontier.insert((Reverse(score), *entry));
                    top_scores.insert((score, *entry));
                    if top_scores.len() > ef {
                        pop_worst_score(&mut top_scores);
                    }
                }
            }
        }

        while let Some((_, candidate)) = pop_best_frontier(&mut frontier) {
            if !visited.insert(candidate) {
                continue;
            }
            let Some(vector) = self.vectors.get(&candidate) else {
                continue;
            };
            let Some(score) = self.config.metric.distance(query, vector) else {
                continue;
            };
            let expand = top_scores.len() < ef
                || score > top_scores.iter().next().map(|item| item.0).unwrap_or(0);
            if !self.deleted.contains(&candidate) {
                scores.insert(candidate, score);
                if expand {
                    top_scores.insert((score, candidate));
                    if top_scores.len() > ef {
                        pop_worst_score(&mut top_scores);
                    }
                }
            }
            if expand {
                if let Some(neighbors) = adjacency.get(&candidate) {
                    for neighbor in neighbors {
                        push_candidate(
                            query,
                            *neighbor,
                            &self.vectors,
                            self.config.metric,
                            &visited,
                            &self.deleted,
                            &mut frontier,
                        );
                    }
                }
            }
        }

        ranked(scores, ef)
            .into_iter()
            .map(|candidate| candidate.cell_id)
            .collect()
    }

    /// HNSW neighbor-selection heuristic (Malkov & Yashunin): keep a
    /// distance-sorted candidate only if it is closer to `base` than to every
    /// already-kept neighbor, preserving the long-range edges that make the graph
    /// navigable. `keep_pruned` backfills toward `m` with the nearest pruned
    /// candidates — used for the NEW node (so it gets enough out-edges in low
    /// dimensions) but NOT when pruning an existing neighbor (so its long-range
    /// bridges are never evicted by nearer edges).
    fn select_neighbors_heuristic(
        &self,
        base: &[i16],
        candidates: &[u32],
        m: usize,
        keep_pruned: bool,
    ) -> Vec<u32> {
        let mut scored: Vec<(u64, u32)> = candidates
            .iter()
            .filter(|id| !self.deleted.contains(id))
            .filter_map(|id| {
                self.vectors
                    .get(id)
                    .and_then(|vector| self.config.metric.distance(base, vector))
                    .map(|score| (score, *id))
            })
            .collect();
        scored.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));

        let mut result: Vec<u32> = Vec::new();
        let mut pruned: Vec<u32> = Vec::new();
        for (score_to_base, candidate) in scored {
            if result.len() >= m {
                break;
            }
            let Some(candidate_vector) = self.vectors.get(&candidate) else {
                continue;
            };
            let redundant = result.iter().any(|kept| {
                self.vectors
                    .get(kept)
                    .and_then(|kept_vector| {
                        self.config.metric.distance(candidate_vector, kept_vector)
                    })
                    .is_some_and(|score_to_kept| score_to_kept > score_to_base)
            });
            if redundant {
                if keep_pruned {
                    pruned.push(candidate);
                }
            } else {
                result.push(candidate);
            }
        }
        if keep_pruned {
            for candidate in pruned {
                if result.len() >= m {
                    break;
                }
                result.push(candidate);
            }
        }
        result
    }

    fn connect_and_prune(&mut self, cell_id: u32, layer: u32, selected: &[u32]) {
        let degree = self.layer_degree(layer);
        {
            let adjacency = self.layer_adjacency_mut(layer);
            adjacency.insert(cell_id, selected.iter().copied().collect());
            for neighbor in selected {
                adjacency.entry(*neighbor).or_default().insert(cell_id);
            }
        }
        for neighbor in selected {
            self.prune_neighbor(*neighbor, layer, degree);
        }
    }

    fn layer_adjacency_mut(&mut self, layer: u32) -> &mut BTreeMap<u32, BTreeSet<u32>> {
        if layer == 0 {
            &mut self.links
        } else {
            self.upper_links.entry(layer).or_default()
        }
    }

    fn prune_neighbor(&mut self, neighbor: u32, layer: u32, degree: usize) {
        let Some(neighbor_vector) = self.vectors.get(&neighbor).cloned() else {
            return;
        };
        let current: Vec<u32> = {
            let adjacency = self.layer_adjacency_mut(layer);
            match adjacency.get(&neighbor) {
                Some(links) if links.len() > degree => links.iter().copied().collect(),
                _ => return,
            }
        };
        // Pure diversity (keep_pruned = false): never backfill bridges away.
        let kept: BTreeSet<u32> = self
            .select_neighbors_heuristic(&neighbor_vector, &current, degree, false)
            .into_iter()
            .collect();
        self.layer_adjacency_mut(layer).insert(neighbor, kept);
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

    /// Exact full-scan nearest neighbors — the O(N) oracle the graph-descent
    /// insert replaced. Kept only as ground truth for recall tests.
    #[cfg(test)]
    pub(crate) fn nearest_existing(&self, vector: &[i16], limit: usize) -> Vec<u32> {
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

#[cfg(test)]
mod tests {
    include!("index/tests.rs");
}
