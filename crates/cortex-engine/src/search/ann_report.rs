use std::collections::{BTreeMap, BTreeSet};
use std::time::Instant;

use crate::error::{EngineError, EngineResult};

use super::ann::{evaluate_persisted_ann_with_policy, AnnSearchPolicy, MIN_ANN_RECALL_Q16};
use super::hnsw::{DistanceMetric, HnswIndex, VectorCollectionConfig};

pub const SYNTHETIC_ANN_CORPUS_V1: &str = "synthetic-ann-corpus-v1";

#[derive(Clone, Debug, PartialEq, Eq)]
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
    pub max_latency_nanos: u128,
    pub production_safe: bool,
}

impl AnnRecallLatencyReport {
    pub fn as_json(&self) -> String {
        format!(
            concat!(
                "{{\"corpus\":\"{}\",\"vector_count\":{},\"dimension\":{},",
                "\"query_count\":{},\"limit\":{},\"graph_nodes\":{},",
                "\"graph_edges\":{},\"upper_layers\":{},\"upper_graph_edges\":{},",
                "\"min_recall_q16\":{},\"min_observed_recall_q16\":{},",
                "\"mean_recall_q16\":{},\"p50_latency_nanos\":{},",
                "\"p95_latency_nanos\":{},\"max_latency_nanos\":{},",
                "\"production_safe\":{}}}"
            ),
            self.corpus,
            self.vector_count,
            self.dimension,
            self.query_count,
            self.limit,
            self.graph_nodes,
            self.graph_edges,
            self.upper_layers,
            self.upper_graph_edges,
            self.min_recall_q16,
            self.min_observed_recall_q16,
            self.mean_recall_q16,
            self.p50_latency_nanos,
            self.p95_latency_nanos,
            self.max_latency_nanos,
            self.production_safe
        )
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
    for query_index in 0..query_count {
        let candidate = (((query_index as u128 * 37) % u128::from(vector_count_u32)) + 1) as u32;
        let query = synthetic_vector(candidate, dimension);
        let started = Instant::now();
        let report =
            evaluate_persisted_ann_with_policy(&vectors, &graph, &query, &allowed, limit, policy);
        latencies.push(started.elapsed().as_nanos());
        recalls.push(report.recall_q16);
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
        max_latency_nanos: *latencies.last().unwrap_or(&0),
        production_safe,
    })
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
        assert!(report
            .as_json()
            .contains("\"corpus\":\"synthetic-ann-corpus-v1\""));
    }
}
