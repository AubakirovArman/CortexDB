use std::collections::BTreeSet;
use std::time::Instant;

use serde::{Deserialize, Serialize};

use crate::error::{EngineError, EngineResult};

use super::ann::{AnnSearchPolicy, MIN_ANN_RECALL_Q16};
use super::ann_external::parse_ann_jsonl_fixture;
use super::hnsw::{DistanceMetric, HnswIndex, VectorCollectionConfig};
use super::persisted::search_persisted_vectors;

use self::compare::compare_metric_matrix_baseline;

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct AnnMetricMatrixBaseline {
    pub baseline_id: String,
    pub fixture_id: String,
    pub fixture_path: String,
    pub policy_min_recall_q16: u16,
    pub require_slo: bool,
    pub metrics: Vec<AnnMetricBaseline>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct AnnMetricBaseline {
    pub metric: String,
    pub min_observed_recall_q16: u16,
    pub min_mean_recall_q16: u16,
    pub min_graph_nodes: usize,
    pub min_graph_edges: usize,
    pub min_upper_layers: usize,
    pub min_upper_graph_edges: usize,
    pub max_p95_latency_nanos: u128,
    pub max_p99_latency_nanos: u128,
    pub max_max_latency_nanos: u128,
    pub require_production_safe: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct AnnMetricMatrixReport {
    pub baseline_id: String,
    pub fixture_id: String,
    pub passed: bool,
    pub failures: Vec<String>,
    pub metrics: Vec<AnnMetricReport>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct AnnMetricReport {
    pub metric: String,
    pub vector_count: usize,
    pub query_count: usize,
    pub dimension: usize,
    pub graph_nodes: usize,
    pub graph_edges: usize,
    pub upper_layers: usize,
    pub upper_graph_edges: usize,
    pub min_observed_recall_q16: u16,
    pub mean_recall_q16: u16,
    pub p50_latency_nanos: u128,
    pub p95_latency_nanos: u128,
    pub p99_latency_nanos: u128,
    pub max_latency_nanos: u128,
    pub production_safe: bool,
}

impl AnnMetricMatrixReport {
    pub fn as_json(&self) -> String {
        serde_json::to_string(self)
            .unwrap_or_else(|_| "{\"error\":\"ann_metric_matrix_serialization_failed\"}".to_owned())
    }
}

pub fn evaluate_ann_metric_matrix(
    baseline: &AnnMetricMatrixBaseline,
    fixture_text: &str,
) -> EngineResult<AnnMetricMatrixReport> {
    if baseline.metrics.is_empty() {
        return Err(EngineError::InvalidAnnFixture(
            "metric matrix baseline must include at least one metric".to_owned(),
        ));
    }
    let fixture = parse_ann_jsonl_fixture(fixture_text)?;
    let allowed = fixture.vectors.keys().copied().collect::<BTreeSet<_>>();
    let policy = AnnSearchPolicy {
        min_recall_q16: Some(baseline.policy_min_recall_q16),
        require_slo: baseline.require_slo,
        ..AnnSearchPolicy::default()
    };
    let mut reports = Vec::with_capacity(baseline.metrics.len());
    for metric_baseline in &baseline.metrics {
        let metric = parse_metric(&metric_baseline.metric)?;
        let mut index = HnswIndex::new_multilayer(8, 64, 4);
        index.set_config(VectorCollectionConfig {
            dimension: fixture.dimension,
            metric,
        });
        for (candidate, vector) in &fixture.vectors {
            index.add_vector(*candidate, vector.clone())?;
        }
        let graph = index.graph_index();
        let persisted_index = HnswIndex::from_graph(fixture.vectors.clone(), graph.clone(), 8, 64);
        let mut recalls = Vec::with_capacity(fixture.queries.len());
        let mut latencies = Vec::with_capacity(fixture.queries.len());
        let mut production_safe = persisted_index.verify_hnsw_integrity();
        for query in &fixture.queries {
            let started = Instant::now();
            let exact = search_persisted_vectors(
                &fixture.vectors,
                &query.vector,
                &allowed,
                query.limit,
                &metric,
            );
            let (ann, _visited, budget_exceeded) = persisted_index.search_allowed_with_budget(
                &query.vector,
                &allowed,
                query.limit,
                None,
            );
            latencies.push(started.elapsed().as_nanos());
            let recall = recall_q16(&ann, &exact);
            production_safe &=
                !budget_exceeded && recall >= policy.min_recall_q16.unwrap_or(MIN_ANN_RECALL_Q16);
            recalls.push(recall);
        }
        reports.push(metric_report(
            metric_baseline.metric.clone(),
            &fixture,
            &graph,
            recalls,
            latencies,
            production_safe,
        ));
    }
    let failures = compare_metric_matrix_baseline(baseline, &reports);
    Ok(AnnMetricMatrixReport {
        baseline_id: baseline.baseline_id.clone(),
        fixture_id: baseline.fixture_id.clone(),
        passed: failures.is_empty(),
        failures,
        metrics: reports,
    })
}

fn metric_report(
    metric: String,
    fixture: &super::ann_external::AnnExternalFixture,
    graph: &cortex_storage::hnsw::HnswGraphIndex,
    mut recalls: Vec<u16>,
    mut latencies: Vec<u128>,
    production_safe: bool,
) -> AnnMetricReport {
    recalls.sort_unstable();
    latencies.sort_unstable();
    let mean_recall_q16 =
        (recalls.iter().copied().map(u64::from).sum::<u64>() / recalls.len() as u64) as u16;
    AnnMetricReport {
        metric,
        vector_count: fixture.vectors.len(),
        query_count: fixture.queries.len(),
        dimension: fixture.dimension,
        graph_nodes: graph.links.len(),
        graph_edges: graph.links.values().map(|neighbors| neighbors.len()).sum(),
        upper_layers: graph.upper_layers.len(),
        upper_graph_edges: graph
            .upper_layers
            .values()
            .flat_map(|links| links.values())
            .map(|neighbors| neighbors.len())
            .sum(),
        min_observed_recall_q16: recalls[0],
        mean_recall_q16,
        p50_latency_nanos: percentile(&latencies, 50),
        p95_latency_nanos: percentile(&latencies, 95),
        p99_latency_nanos: percentile(&latencies, 99),
        max_latency_nanos: *latencies.last().unwrap_or(&0),
        production_safe,
    }
}

fn parse_metric(value: &str) -> EngineResult<DistanceMetric> {
    match value {
        "dot_product" | "dotproduct" | "dot" => Ok(DistanceMetric::DotProduct),
        "cosine" => Ok(DistanceMetric::Cosine),
        "l2" | "euclidean" => Ok(DistanceMetric::L2),
        _ => Err(EngineError::InvalidAnnFixture(format!(
            "unknown metric {value}"
        ))),
    }
}

fn recall_q16(ann: &[super::ScoredCandidate], exact: &[super::ScoredCandidate]) -> u16 {
    if exact.is_empty() {
        return 65_535;
    }
    let exact_ids = exact
        .iter()
        .map(|candidate| candidate.cell_id)
        .collect::<BTreeSet<_>>();
    let overlap = ann
        .iter()
        .filter(|candidate| exact_ids.contains(&candidate.cell_id))
        .count();
    ((overlap as u64 * 65_535) / exact.len() as u64) as u16
}

fn percentile(values: &[u128], percentile: usize) -> u128 {
    if values.is_empty() {
        return 0;
    }
    values[((values.len() - 1) * percentile.min(100)) / 100]
}

mod compare;

#[cfg(test)]
mod tests;
