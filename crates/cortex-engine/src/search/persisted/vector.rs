use std::collections::{BTreeMap, BTreeSet};

use cortex_storage::vectors::VectorIndexReader;

use crate::error::EngineResult;

use super::super::{hnsw::DistanceMetric, ranked, ScoredCandidate};

pub(in crate::search) fn search_persisted_vectors(
    vectors: &BTreeMap<u32, Vec<i16>>,
    query: &[i16],
    allowed: &BTreeSet<u32>,
    limit: usize,
    metric: &DistanceMetric,
) -> Vec<ScoredCandidate> {
    let scores = vectors
        .iter()
        .filter(|(candidate, _)| allowed.contains(candidate))
        .filter_map(|(candidate, vector)| {
            vector_score(metric, query, vector).map(|score| (*candidate, score))
        })
        .collect();
    ranked(scores, limit)
}

pub(in crate::search) fn search_persisted_vector_reader(
    reader: &mut VectorIndexReader,
    query: &[i16],
    allowed: &BTreeSet<u32>,
    limit: usize,
    metric: &DistanceMetric,
) -> EngineResult<Vec<ScoredCandidate>> {
    let scores =
        reader.scan_scores(allowed, limit, |vector| vector_score(metric, query, vector))?;
    Ok(scores
        .into_iter()
        .map(|(cell_id, score)| ScoredCandidate { cell_id, score })
        .collect())
}

fn vector_score(metric: &DistanceMetric, query: &[i16], vector: &[i16]) -> Option<u64> {
    if *metric == DistanceMetric::DotProduct {
        dot_product_i16(query, vector)
    } else {
        metric.distance(query, vector)
    }
}

fn dot_product_i16(query: &[i16], vector: &[i16]) -> Option<u64> {
    if query.len() != vector.len() {
        return None;
    }
    let mut sum = 0_i64;
    let mut query_chunks = query.chunks_exact(8);
    let mut vector_chunks = vector.chunks_exact(8);
    for (left, right) in query_chunks.by_ref().zip(vector_chunks.by_ref()) {
        sum += i64::from(left[0]) * i64::from(right[0])
            + i64::from(left[1]) * i64::from(right[1])
            + i64::from(left[2]) * i64::from(right[2])
            + i64::from(left[3]) * i64::from(right[3])
            + i64::from(left[4]) * i64::from(right[4])
            + i64::from(left[5]) * i64::from(right[5])
            + i64::from(left[6]) * i64::from(right[6])
            + i64::from(left[7]) * i64::from(right[7]);
    }
    for (left, right) in query_chunks
        .remainder()
        .iter()
        .zip(vector_chunks.remainder())
    {
        sum += i64::from(*left) * i64::from(*right);
    }
    Some(sum.max(0) as u64)
}
