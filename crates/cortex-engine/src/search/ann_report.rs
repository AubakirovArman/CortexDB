use std::collections::{BTreeMap, BTreeSet};
use std::time::Instant;

use serde::Serialize;

use crate::error::{EngineError, EngineResult};
use cortex_storage::hnsw::HnswGraphIndex;

use super::ann::{evaluate_persisted_ann_with_policy, AnnSearchPolicy, MIN_ANN_RECALL_Q16};
use super::hnsw::{DistanceMetric, HnswIndex, VectorCollectionConfig};

pub const SYNTHETIC_ANN_CORPUS_V1: &str = "synthetic-ann-corpus-v1";

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct AnnRecallLatencyReport {
    pub corpus: &'static str,
    pub vector_count: usize,
    pub dimension: usize,
    pub query_count: usize,
    pub limit: usize,
    pub graph_nodes: usize,
    pub graph_edges: usize,
    pub upper_layers: usize,
    pub upper_graph_edges: usize,
    pub min_recall_q16: u16,
    pub min_observed_recall_q16: u16,
    pub mean_recall_q16: u16,
    pub p50_latency_nanos: u128,
    pub p95_latency_nanos: u128,
    pub p99_latency_nanos: u128,
    pub max_latency_nanos: u128,
    pub hnsw_max_neighbors: usize,
    pub hnsw_ef_search: usize,
    pub hnsw_layer_count: usize,
    pub hnsw_ef_construction: usize,
    pub graph_signature: String,
    pub fallback_count: usize,
    pub fallback_rate_q16: u16,
    pub production_safe: bool,
}

impl AnnRecallLatencyReport {
    pub fn as_json(&self) -> String {
        serde_json::to_string(self)
            .unwrap_or_else(|_| "{\"error\":\"ann_report_serialization_failed\"}".to_owned())
    }
}

pub fn synthetic_ann_recall_latency_report(
    vector_count: usize,
    dimension: usize,
    query_count: usize,
    limit: usize,
    policy: AnnSearchPolicy,
) -> EngineResult<AnnRecallLatencyReport> {
    let vector_count_u32 =
        u32::try_from(vector_count.max(1)).map_err(|_| EngineError::CandidateIdOverflow)?;
    let vector_count = vector_count_u32 as usize;
    let dimension = dimension.max(1);
    let query_count = query_count.max(1);
    let limit = limit.max(1);
    let mut index = HnswIndex::new_multilayer(8, 64, 4);
    index.set_config(VectorCollectionConfig {
        dimension,
        metric: DistanceMetric::DotProduct,
    });
    let mut vectors = BTreeMap::new();
    for candidate in 1..=vector_count_u32 {
        let vector = synthetic_vector(candidate, dimension);
        index.add_vector(candidate, vector.clone())?;
        vectors.insert(candidate, vector);
    }
    let graph = index.graph_index();
    let allowed = vectors.keys().copied().collect::<BTreeSet<_>>();
    let mut recalls = Vec::with_capacity(query_count);
    let mut latencies = Vec::with_capacity(query_count);
    let mut production_safe = true;
    let mut fallback_count = 0usize;
    for query_index in 0..query_count {
        let candidate = (((query_index as u128 * 37) % u128::from(vector_count_u32)) + 1) as u32;
        let query = synthetic_vector(candidate, dimension);
        let started = Instant::now();
        let report =
            evaluate_persisted_ann_with_policy(&vectors, &graph, &query, &allowed, limit, policy);
        latencies.push(started.elapsed().as_nanos());
        recalls.push(report.recall_q16);
        if report.search.fallback_performed {
            fallback_count += 1;
        }
        production_safe &= report.search.production_safe;
    }
    latencies.sort_unstable();
    recalls.sort_unstable();
    let mean_recall_q16 =
        (recalls.iter().copied().map(u64::from).sum::<u64>() / recalls.len() as u64) as u16;
    Ok(AnnRecallLatencyReport {
        corpus: SYNTHETIC_ANN_CORPUS_V1,
        vector_count,
        dimension,
        query_count,
        limit,
        graph_nodes: graph.links.len(),
        graph_edges: graph.links.values().map(|neighbors| neighbors.len()).sum(),
        upper_layers: graph.upper_layers.len(),
        upper_graph_edges: graph
            .upper_layers
            .values()
            .flat_map(|links| links.values())
            .map(|neighbors| neighbors.len())
            .sum(),
        min_recall_q16: policy.min_recall_q16.unwrap_or(MIN_ANN_RECALL_Q16),
        min_observed_recall_q16: recalls[0],
        mean_recall_q16,
        p50_latency_nanos: percentile(&latencies, 50),
        p95_latency_nanos: percentile(&latencies, 95),
        p99_latency_nanos: percentile(&latencies, 99),
        max_latency_nanos: *latencies.last().unwrap_or(&0),
        hnsw_max_neighbors: if graph.max_neighbors == 0 {
            8
        } else {
            graph.max_neighbors as usize
        },
        hnsw_ef_search: if graph.ef_search == 0 {
            64
        } else {
            graph.ef_search as usize
        },
        hnsw_layer_count: if graph.layer_count == 0 {
            graph
                .upper_layers
                .keys()
                .next_back()
                .and_then(|layer| (*layer as usize).checked_add(1))
                .unwrap_or(1)
        } else {
            graph.layer_count as usize
        },
        hnsw_ef_construction: if graph.ef_construction == 0 {
            graph.ef_search.max(64) as usize
        } else {
            graph.ef_construction as usize
        },
        graph_signature: graph_signature(&graph),
        fallback_count,
        fallback_rate_q16: ratio_q16(fallback_count, query_count),
        production_safe,
    })
}

fn ratio_q16(numerator: usize, denominator: usize) -> u16 {
    if denominator == 0 {
        return 65_535;
    }
    ((numerator as u64 * 65_535) / denominator as u64) as u16
}

fn graph_signature(graph: &HnswGraphIndex) -> String {
    let mut hash = 0xcbf29ce484222325u64;

    fn absorb_u64(hash: &mut u64, value: u64) {
        for byte in value.to_le_bytes() {
            *hash ^= u64::from(byte);
            *hash = hash.wrapping_mul(0x0000_0100_0000_01B3);
        }
    }

    fn absorb_pair(hash: &mut u64, left: u64, right: u64) {
        absorb_u64(hash, left);
        absorb_u64(hash, right);
    }

    absorb_pair(
        &mut hash,
        u64::from(graph.dimension),
        u64::from(graph.metric),
    );
    absorb_pair(
        &mut hash,
        u64::from(graph.max_neighbors),
        u64::from(graph.ef_search),
    );
    absorb_u64(
        &mut hash,
        u64::from(graph.ef_construction.max(graph.ef_search)),
    );
    absorb_u64(&mut hash, u64::from(graph.layer_count));

    for (candidate, neighbors) in &graph.links {
        absorb_u64(&mut hash, u64::from(*candidate));
        absorb_u64(&mut hash, neighbors.len() as u64);
        for neighbor in neighbors {
            absorb_u64(&mut hash, u64::from(*neighbor));
        }
    }

    for (layer, links) in &graph.upper_layers {
        absorb_u64(&mut hash, u64::from(*layer));
        for (candidate, neighbors) in links {
            absorb_u64(&mut hash, u64::from(*candidate));
            absorb_u64(&mut hash, neighbors.len() as u64);
            for neighbor in neighbors {
                absorb_u64(&mut hash, u64::from(*neighbor));
            }
        }
    }

    format!("{hash:016x}")
}

fn synthetic_vector(candidate: u32, dimension: usize) -> Vec<i16> {
    (0..dimension)
        .map(|offset| ((u128::from(candidate) * (31 + offset as u128 * 17)) % 1024) as i16)
        .collect()
}

fn percentile(values: &[u128], percentile: usize) -> u128 {
    if values.is_empty() {
        return 0;
    }
    let index = ((values.len() - 1) * percentile.min(100)) / 100;
    values[index]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn synthetic_ann_report_is_repeatable_in_shape_and_corpus() {
        let report = synthetic_ann_recall_latency_report(
            128,
            8,
            8,
            10,
            AnnSearchPolicy {
                require_slo: true,
                ..AnnSearchPolicy::default()
            },
        )
        .unwrap();

        assert_eq!(report.corpus, SYNTHETIC_ANN_CORPUS_V1);
        assert_eq!(report.vector_count, 128);
        assert_eq!(report.dimension, 8);
        assert_eq!(report.query_count, 8);
        assert!(report.graph_nodes > 0);
        assert!(report.upper_layers > 0);
        assert!(report.max_latency_nanos >= report.p50_latency_nanos);
        assert!(report.p99_latency_nanos >= report.p95_latency_nanos);
        assert!(report.max_latency_nanos >= report.p99_latency_nanos);
        assert!(report
            .as_json()
            .contains("\"corpus\":\"synthetic-ann-corpus-v1\""));
        assert_eq!(report.hnsw_max_neighbors, 8);
        assert_eq!(report.hnsw_ef_search, 64);
        assert_eq!(report.hnsw_layer_count, 4);
        assert!(!report.graph_signature.is_empty());
    }

    #[test]
    fn synthetic_ann_report_is_repeatable_with_same_graph_signature() {
        let first = synthetic_ann_recall_latency_report(
            256,
            16,
            12,
            5,
            AnnSearchPolicy {
                require_slo: true,
                ..AnnSearchPolicy::default()
            },
        )
        .unwrap();

        let second = synthetic_ann_recall_latency_report(
            256,
            16,
            12,
            5,
            AnnSearchPolicy {
                require_slo: true,
                ..AnnSearchPolicy::default()
            },
        )
        .unwrap();

        assert_eq!(first.graph_signature, second.graph_signature);
        assert_eq!(first.graph_nodes, second.graph_nodes);
        assert_eq!(first.upper_graph_edges, second.upper_graph_edges);
        assert_eq!(first.hnsw_layer_count, second.hnsw_layer_count);
    }
}
