use std::collections::{BTreeMap, BTreeSet};

use cortex_storage::hnsw::HnswGraphIndex;

use super::{dot_nonnegative, ranked, ScoredCandidate};

#[derive(Clone, Debug)]
pub struct HnswIndex {
    vectors: BTreeMap<u32, Vec<i16>>,
    links: BTreeMap<u32, BTreeSet<u32>>,
    max_neighbors: usize,
    ef_search: usize,
}

impl HnswIndex {
    pub fn new(max_neighbors: usize, ef_search: usize) -> Self {
        Self {
            vectors: BTreeMap::new(),
            links: BTreeMap::new(),
            max_neighbors: max_neighbors.max(1),
            ef_search: ef_search.max(1),
        }
    }

    pub fn add_vector(&mut self, cell_id: u32, vector: Vec<i16>) {
        let neighbors = self.nearest_existing(&vector, self.max_neighbors);
        for neighbor in &neighbors {
            self.links.entry(*neighbor).or_default().insert(cell_id);
        }
        self.links
            .insert(cell_id, neighbors.iter().copied().collect::<BTreeSet<_>>());
        self.vectors.insert(cell_id, vector);
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
            if !visited.insert(candidate) || visited.len() > self.ef_search {
                continue;
            }
            if allowed.is_none_or(|values| values.contains(&candidate)) {
                scores.insert(candidate, dot_nonnegative(query, &self.vectors[&candidate]));
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
            max_neighbors: max_neighbors.max(1),
            ef_search: ef_search.max(1),
        }
    }

    fn nearest_existing(&self, vector: &[i16], limit: usize) -> Vec<u32> {
        let scores = self
            .vectors
            .iter()
            .map(|(cell_id, existing)| (*cell_id, dot_nonnegative(vector, existing)))
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
