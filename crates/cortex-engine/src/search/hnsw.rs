use std::collections::{BTreeMap, BTreeSet};

use cortex_storage::hnsw::HnswGraphIndex;

use super::hnsw_policy::{HnswMaintenancePolicy, HnswMaintenanceReport, HnswRebuildPolicy};
use super::{ranked, ScoredCandidate};

pub mod integrity;

/// Supported distance metrics for vector similarity search.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum DistanceMetric {
    /// Non-negative dot-product similarity. Higher is better.
    #[default]
    DotProduct,
    /// Cosine similarity scaled to [0, 65_535]. Higher is better.
    Cosine,
    /// Negative squared Euclidean distance. Higher (less negative) is better.
    L2,
}

impl DistanceMetric {
    /// Compute similarity between two vectors. Returns `None` if dimensions mismatch.
    pub fn distance(&self, u: &[i16], v: &[i16]) -> Option<u64> {
        if u.len() != v.len() {
            return None;
        }
        match self {
            Self::DotProduct => Some(
                u.iter()
                    .zip(v)
                    .map(|(left, right)| i64::from(*left) * i64::from(*right))
                    .sum::<i64>()
                    .max(0) as u64,
            ),
            Self::Cosine => {
                let dot: i64 = u
                    .iter()
                    .zip(v)
                    .map(|(a, b)| i64::from(*a) * i64::from(*b))
                    .sum();
                let u_norm_sq: i64 = u.iter().map(|x| i64::from(*x) * i64::from(*x)).sum();
                let v_norm_sq: i64 = v.iter().map(|x| i64::from(*x) * i64::from(*x)).sum();
                if u_norm_sq == 0 || v_norm_sq == 0 {
                    return Some(0);
                }
                let norm = ((u_norm_sq as f64).sqrt() * (v_norm_sq as f64).sqrt()) as i64;
                if norm == 0 {
                    return Some(0);
                }
                Some(((dot.abs() * 65_535) / norm.abs()) as u64)
            }
            Self::L2 => {
                let dist_sq: i64 = u
                    .iter()
                    .zip(v)
                    .map(|(a, b)| {
                        let diff = i64::from(*a) - i64::from(*b);
                        diff * diff
                    })
                    .sum();
                let max_dist = (u.len() as i64) * 65_536i64 * 65_536i64;
                Some((max_dist - dist_sq.min(max_dist)).max(0) as u64)
            }
        }
    }
}

/// Configuration for a vector collection persisted alongside the HNSW graph.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct VectorCollectionConfig {
    pub dimension: usize,
    pub metric: DistanceMetric,
}

#[derive(Clone, Debug)]
pub struct HnswIndex {
    vectors: BTreeMap<u32, Vec<i16>>,
    links: BTreeMap<u32, BTreeSet<u32>>,
    deleted: BTreeSet<u32>,
    layer_count: usize,
    max_neighbors: usize,
    ef_search: usize,
    config: VectorCollectionConfig,
    pub rebuild_count: u64,
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

    pub fn add_vector(&mut self, cell_id: u32, vector: Vec<i16>) {
        if self.config.dimension > 0 && vector.len() != self.config.dimension {
            return;
        }
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
        self.rebuild_count += 1;
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
        while let Some(candidate) =
            best_frontier(query, &frontier, &self.vectors, &self.config.metric)
        {
            frontier.remove(&candidate);
            if self.deleted.contains(&candidate)
                || !visited.insert(candidate)
                || visited.len() > self.ef_search
            {
                continue;
            }
            if allowed.is_none_or(|values| values.contains(&candidate)) {
                if let Some(score) = self
                    .config
                    .metric
                    .distance(query, &self.vectors[&candidate])
                {
                    scores.insert(candidate, score);
                }
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
            dimension: self.config.dimension as u32,
            metric: self.config.metric as u8,
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
        Self {
            vectors,
            links: graph.links,
            deleted: BTreeSet::new(),
            layer_count: 1,
            max_neighbors: max_neighbors.max(1),
            ef_search: ef_search.max(1),
            config: VectorCollectionConfig { dimension, metric },
            rebuild_count: 0,
        }
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
    metric: &DistanceMetric,
) -> Option<u32> {
    frontier
        .iter()
        .copied()
        .filter_map(|cell_id| {
            metric
                .distance(query, &vectors[&cell_id])
                .map(|score| (cell_id, score))
        })
        .max_by_key(|(_, score)| *score)
        .map(|(cell_id, _)| cell_id)
}
