use std::collections::{BTreeMap, BTreeSet};

use cortex_storage::hnsw::HnswGraphIndex;

use super::hnsw_policy::{HnswMaintenancePolicy, HnswMaintenanceReport, HnswRebuildPolicy};
use super::{dot_nonnegative, ranked, ScoredCandidate};

pub mod integrity;

/// Abstraction for distance calculation between high-dimensional vector embeddings.
pub trait DistanceMetric {
    /// Calculate the similarity score (distance) between two vector embeddings.
    fn distance(u: &[i16], v: &[i16]) -> u64;
}

/// Dot-product similarity implementation of DistanceMetric.
pub struct DotProductMetric;

impl DistanceMetric for DotProductMetric {
    fn distance(u: &[i16], v: &[i16]) -> u64 {
        dot_nonnegative(u, v)
    }
}

#[derive(Clone, Debug)]
pub struct HnswIndex {
    vectors: BTreeMap<u32, Vec<i16>>,
    links: BTreeMap<u32, BTreeSet<u32>>,
    deleted: BTreeSet<u32>,
    layer_count: usize,
    max_neighbors: usize,
    ef_search: usize,
}

impl HnswIndex {
    pub fn new(max_neighbors: usize, ef_search: usize) -> Self {
        Self {
            vectors: BTreeMap::new(),
            links: BTreeMap::new(),
            deleted: BTreeSet::new(),
            layer_count: 1,
            max_neighbors: max_neighbors.max(1),
            ef_search: ef_search.max(1),
        }
    }

    pub fn new_multilayer(max_neighbors: usize, ef_search: usize, layer_count: usize) -> Self {
        Self {
            layer_count: layer_count.max(1),
            ..Self::new(max_neighbors, ef_search)
        }
    }

    pub fn add_vector(&mut self, cell_id: u32, vector: Vec<i16>) {
        self.deleted.remove(&cell_id);
        let neighbors = self.nearest_existing(&vector, self.max_neighbors);
        for neighbor in &neighbors {
            self.links.entry(*neighbor).or_default().insert(cell_id);
        }
        self.links
            .insert(cell_id, neighbors.iter().copied().collect::<BTreeSet<_>>());
        self.vectors.insert(cell_id, vector);
    }

    pub fn remove_vector(&mut self, cell_id: u32) -> bool {
        if self.vectors.contains_key(&cell_id) {
            self.deleted.insert(cell_id);
            self.links.remove(&cell_id);
            for neighbors in self.links.values_mut() {
                neighbors.remove(&cell_id);
            }
            true
        } else {
            false
        }
    }

    pub fn rebuild_if_needed(&mut self, policy: HnswRebuildPolicy) -> bool {
        if !self.should_rebuild(policy) {
            return false;
        }
        self.vectors
            .retain(|candidate, _| !self.deleted.contains(candidate));
        self.deleted.clear();
        self.rebuild_links();
        true
    }

    pub fn apply_maintenance(&mut self, policy: HnswMaintenancePolicy) -> HnswMaintenanceReport {
        let vectors_before = self.vectors.len();
        let deleted_before = self.deleted.len();
        let rebuilt = deleted_before >= policy.min_deleted_vectors
            && self.rebuild_if_needed(policy.rebuild_policy);
        HnswMaintenanceReport {
            vectors_before,
            deleted_before,
            rebuilt,
        }
    }

    pub fn vector_count(&self) -> usize {
        self.vectors.len()
    }

    pub fn deleted_count(&self) -> usize {
        self.deleted.len()
    }

    pub fn maintenance_due(&self, policy: HnswMaintenancePolicy) -> bool {
        self.deleted.len() >= policy.min_deleted_vectors
            && self.should_rebuild(policy.rebuild_policy)
    }

    pub fn layer_count(&self) -> usize {
        self.layer_count
    }

    pub fn search(&self, query: &[i16], limit: usize) -> Vec<ScoredCandidate> {
        self.search_filtered(query, None, limit)
    }

    pub fn search_allowed(
        &self,
        query: &[i16],
        allowed: &BTreeSet<u32>,
        limit: usize,
    ) -> Vec<ScoredCandidate> {
        self.search_filtered(query, Some(allowed), limit)
    }

    fn search_filtered(
        &self,
        query: &[i16],
        allowed: Option<&BTreeSet<u32>>,
        limit: usize,
    ) -> Vec<ScoredCandidate> {
        let Some(entry) = self.vectors.keys().next().copied() else {
            return Vec::new();
        };
        let mut visited = BTreeSet::new();
        let mut frontier = BTreeSet::from([entry]);
        let mut scores = BTreeMap::new();
        while let Some(candidate) = best_frontier(query, &frontier, &self.vectors) {
            frontier.remove(&candidate);
            if self.deleted.contains(&candidate)
                || !visited.insert(candidate)
                || visited.len() > self.ef_search
            {
                continue;
            }
            if allowed.is_none_or(|values| values.contains(&candidate)) {
                scores.insert(
                    candidate,
                    DotProductMetric::distance(query, &self.vectors[&candidate]),
                );
            }
            if let Some(neighbors) = self.links.get(&candidate) {
                for neighbor in neighbors {
                    if !visited.contains(neighbor) {
                        frontier.insert(*neighbor);
                    }
                }
            }
        }
        ranked(scores, limit)
    }

    pub fn graph_index(&self) -> HnswGraphIndex {
        HnswGraphIndex {
            links: self.links.clone(),
        }
    }

    pub fn from_graph(
        vectors: BTreeMap<u32, Vec<i16>>,
        graph: HnswGraphIndex,
        max_neighbors: usize,
        ef_search: usize,
    ) -> Self {
        Self {
            vectors,
            links: graph.links,
            deleted: BTreeSet::new(),
            layer_count: 1,
            max_neighbors: max_neighbors.max(1),
            ef_search: ef_search.max(1),
        }
    }

    fn nearest_existing(&self, vector: &[i16], limit: usize) -> Vec<u32> {
        let scores = self
            .vectors
            .iter()
            .filter(|(cell_id, _)| !self.deleted.contains(cell_id))
            .map(|(cell_id, existing)| (*cell_id, DotProductMetric::distance(vector, existing)))
            .collect();
        ranked(scores, limit)
            .into_iter()
            .map(|candidate| candidate.cell_id)
            .collect()
    }

    fn should_rebuild(&self, policy: HnswRebuildPolicy) -> bool {
        if self.deleted.is_empty() || self.vectors.is_empty() {
            return false;
        }
        let deleted_q16 = (self.deleted.len() as u64 * 65_535) / self.vectors.len() as u64;
        deleted_q16 >= u64::from(policy.deleted_fraction_q16)
    }

    fn rebuild_links(&mut self) {
        let vectors = self.vectors.clone();
        self.links.clear();
        for (cell_id, vector) in vectors {
            let neighbors = self.nearest_existing_except(cell_id, &vector, self.max_neighbors);
            for neighbor in &neighbors {
                self.links.entry(*neighbor).or_default().insert(cell_id);
            }
            self.links.insert(cell_id, neighbors.into_iter().collect());
        }
    }

    fn nearest_existing_except(&self, skip: u32, vector: &[i16], limit: usize) -> Vec<u32> {
        let scores = self
            .vectors
            .iter()
            .filter(|(cell_id, _)| **cell_id != skip)
            .map(|(cell_id, existing)| (*cell_id, DotProductMetric::distance(vector, existing)))
            .collect();
        ranked(scores, limit)
            .into_iter()
            .map(|candidate| candidate.cell_id)
            .collect()
    }
}

impl Default for HnswIndex {
    fn default() -> Self {
        Self::new(8, 32)
    }
}

fn best_frontier(
    query: &[i16],
    frontier: &BTreeSet<u32>,
    vectors: &BTreeMap<u32, Vec<i16>>,
) -> Option<u32> {
    frontier
        .iter()
        .copied()
        .max_by_key(|cell_id| dot_nonnegative(query, &vectors[cell_id]))
}
