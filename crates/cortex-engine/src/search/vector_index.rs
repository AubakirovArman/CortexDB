use std::collections::BTreeMap;

use super::{hnsw, ranked, ScoredCandidate};

#[derive(Clone, Debug, Default)]
pub struct VectorIndex {
    vectors: BTreeMap<u32, Vec<i16>>,
    metric: hnsw::DistanceMetric,
}

impl VectorIndex {
    pub fn add_vector(&mut self, cell_id: u32, vector: Vec<i16>) {
        self.vectors.insert(cell_id, vector);
    }

    pub fn search_dot(&self, query: &[i16], limit: usize) -> Vec<ScoredCandidate> {
        let scores = self
            .vectors
            .iter()
            .filter_map(|(cell_id, vector)| {
                self.metric
                    .distance(query, vector)
                    .map(|score| (*cell_id, score))
            })
            .collect();
        ranked(scores, limit)
    }
}
