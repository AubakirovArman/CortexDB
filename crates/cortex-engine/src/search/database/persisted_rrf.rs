use std::cmp::Reverse;
use std::collections::BTreeMap;

use super::super::hnsw::DistanceMetric;
use super::super::{HybridRrfWeights, ScoredCandidate};
use super::PersistedSearchCandidate;

pub(super) fn distance_metric_from_manifest(value: u32) -> DistanceMetric {
    match value {
        1 => DistanceMetric::Cosine,
        2 => DistanceMetric::L2,
        _ => DistanceMetric::DotProduct,
    }
}

pub(super) fn fuse_persisted_rrf(
    lexical: Vec<ScoredCandidate>,
    vector: Vec<ScoredCandidate>,
    limit: usize,
    weights: HybridRrfWeights,
) -> Vec<PersistedSearchCandidate> {
    let mut results = BTreeMap::<u32, PersistedSearchCandidate>::new();
    apply_persisted_rrf(&mut results, lexical, true, weights.lexical_q16);
    apply_persisted_rrf(&mut results, vector, false, weights.vector_q16);
    let mut values = results.into_values().collect::<Vec<_>>();
    values.sort_by_key(|candidate| (Reverse(candidate.score), candidate.candidate_id));
    values.truncate(limit);
    values
}

fn apply_persisted_rrf(
    results: &mut BTreeMap<u32, PersistedSearchCandidate>,
    ranked: Vec<ScoredCandidate>,
    lexical: bool,
    weight_q16: u32,
) {
    for (rank, candidate) in ranked.into_iter().enumerate() {
        let rrf =
            (1_000_000 / (60 + rank as u64 + 1)).saturating_mul(u64::from(weight_q16)) / 65_535;
        let result = results
            .entry(candidate.cell_id)
            .or_insert(PersistedSearchCandidate {
                candidate_id: candidate.cell_id,
                score: 0,
                lexical_score: 0,
                vector_score: 0,
            });
        result.score += rrf;
        if lexical {
            result.lexical_score = candidate.score;
        } else {
            result.vector_score = candidate.score;
        }
    }
}
