use serde::{Deserialize, Serialize};

use crate::error::EngineResult;

use super::ann::{AnnSearchPolicy, MIN_ANN_RECALL_Q16};
use super::ann_report::{
    synthetic_ann_recall_latency_report, AnnRecallLatencyReport, SYNTHETIC_ANN_CORPUS_V1,
};

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct AnnRecallLatencyBaseline {
    pub baseline_id: String,
    pub corpus: String,
    pub vector_count: usize,
    pub dimension: usize,
    pub query_count: usize,
    pub limit: usize,
    pub policy_min_recall_q16: u16,
    pub require_slo: bool,
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
pub struct AnnRecallLatencyGateReport {
    pub baseline_id: String,
    pub passed: bool,
    pub failures: Vec<String>,
    pub observed: AnnRecallLatencyReport,
}

impl AnnRecallLatencyBaseline {
    pub fn synthetic_v1() -> Self {
        Self {
            baseline_id: "synthetic-ann-corpus-v1-core-gate".to_owned(),
            corpus: SYNTHETIC_ANN_CORPUS_V1.to_owned(),
            vector_count: 1000,
            dimension: 8,
            query_count: 32,
            limit: 10,
            policy_min_recall_q16: MIN_ANN_RECALL_Q16,
            require_slo: true,
            min_observed_recall_q16: MIN_ANN_RECALL_Q16,
            min_mean_recall_q16: MIN_ANN_RECALL_Q16,
            min_graph_nodes: 1000,
            min_graph_edges: 1000,
            min_upper_layers: 1,
            min_upper_graph_edges: 1,
            max_p95_latency_nanos: 100_000_000,
            max_p99_latency_nanos: 200_000_000,
            max_max_latency_nanos: 250_000_000,
            require_production_safe: true,
        }
    }
}

impl AnnRecallLatencyGateReport {
    pub fn as_json(&self) -> String {
        serde_json::to_string(self)
            .unwrap_or_else(|_| "{\"error\":\"ann_fixture_gate_serialization_failed\"}".to_owned())
    }
}

pub fn evaluate_ann_fixture_baseline(
    baseline: &AnnRecallLatencyBaseline,
) -> EngineResult<AnnRecallLatencyGateReport> {
    let observed = synthetic_ann_recall_latency_report(
        baseline.vector_count,
        baseline.dimension,
        baseline.query_count,
        baseline.limit,
        AnnSearchPolicy {
            min_recall_q16: Some(baseline.policy_min_recall_q16),
            require_slo: baseline.require_slo,
            ..AnnSearchPolicy::default()
        },
    )?;

    let failures = compare_ann_fixture_baseline(baseline, &observed);
    Ok(AnnRecallLatencyGateReport {
        baseline_id: baseline.baseline_id.clone(),
        passed: failures.is_empty(),
        failures,
        observed,
    })
}

pub fn compare_ann_fixture_baseline(
    baseline: &AnnRecallLatencyBaseline,
    observed: &AnnRecallLatencyReport,
) -> Vec<String> {
    let mut failures = Vec::new();
    if observed.corpus != baseline.corpus {
        failures.push(format!(
            "corpus: expected {}, observed {}",
            baseline.corpus, observed.corpus
        ));
    }
    check_eq(
        &mut failures,
        "vector_count",
        baseline.vector_count,
        observed.vector_count,
    );
    check_eq(
        &mut failures,
        "dimension",
        baseline.dimension,
        observed.dimension,
    );
    check_eq(
        &mut failures,
        "query_count",
        baseline.query_count,
        observed.query_count,
    );
    check_eq(&mut failures, "limit", baseline.limit, observed.limit);
    check_min(
        &mut failures,
        "min_observed_recall_q16",
        observed.min_observed_recall_q16,
        baseline.min_observed_recall_q16,
    );
    check_min(
        &mut failures,
        "mean_recall_q16",
        observed.mean_recall_q16,
        baseline.min_mean_recall_q16,
    );
    check_min(
        &mut failures,
        "graph_nodes",
        observed.graph_nodes,
        baseline.min_graph_nodes,
    );
    check_min(
        &mut failures,
        "graph_edges",
        observed.graph_edges,
        baseline.min_graph_edges,
    );
    check_min(
        &mut failures,
        "upper_layers",
        observed.upper_layers,
        baseline.min_upper_layers,
    );
    check_min(
        &mut failures,
        "upper_graph_edges",
        observed.upper_graph_edges,
        baseline.min_upper_graph_edges,
    );
    check_max(
        &mut failures,
        "p95_latency_nanos",
        observed.p95_latency_nanos,
        baseline.max_p95_latency_nanos,
    );
    check_max(
        &mut failures,
        "p99_latency_nanos",
        observed.p99_latency_nanos,
        baseline.max_p99_latency_nanos,
    );
    check_max(
        &mut failures,
        "max_latency_nanos",
        observed.max_latency_nanos,
        baseline.max_max_latency_nanos,
    );
    if baseline.require_production_safe && !observed.production_safe {
        failures.push("production_safe: expected true, observed false".to_owned());
    }
    failures
}

fn check_eq<T>(failures: &mut Vec<String>, field: &str, expected: T, observed: T)
where
    T: std::fmt::Display + PartialEq,
{
    if observed != expected {
        failures.push(format!("{field}: expected {expected}, observed {observed}"));
    }
}

fn check_min<T>(failures: &mut Vec<String>, field: &str, observed: T, minimum: T)
where
    T: std::fmt::Display + PartialOrd,
{
    if observed < minimum {
        failures.push(format!(
            "{field}: expected >= {minimum}, observed {observed}"
        ));
    }
}

fn check_max<T>(failures: &mut Vec<String>, field: &str, observed: T, maximum: T)
where
    T: std::fmt::Display + PartialOrd,
{
    if observed > maximum {
        failures.push(format!(
            "{field}: expected <= {maximum}, observed {observed}"
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn synthetic_baseline_gate_passes() {
        let mut baseline = AnnRecallLatencyBaseline::synthetic_v1();
        baseline.vector_count = 64;
        baseline.query_count = 4;
        baseline.min_graph_nodes = 64;
        baseline.min_graph_edges = 64;

        let report = evaluate_ann_fixture_baseline(&baseline).unwrap();

        assert!(report.passed, "{:?}", report.failures);
        assert_eq!(report.baseline_id, baseline.baseline_id);
    }

    #[test]
    fn baseline_gate_reports_structural_failures() {
        let observed = AnnRecallLatencyReport {
            corpus: SYNTHETIC_ANN_CORPUS_V1,
            vector_count: 10,
            dimension: 8,
            query_count: 2,
            limit: 5,
            graph_nodes: 10,
            graph_edges: 20,
            upper_layers: 1,
            upper_graph_edges: 2,
            min_recall_q16: MIN_ANN_RECALL_Q16,
            min_observed_recall_q16: MIN_ANN_RECALL_Q16,
            mean_recall_q16: MIN_ANN_RECALL_Q16,
            p50_latency_nanos: 1,
            p95_latency_nanos: 1,
            p99_latency_nanos: 1,
            max_latency_nanos: 1,
            hnsw_max_neighbors: 8,
            hnsw_ef_search: 64,
            hnsw_layer_count: 4,
            hnsw_ef_construction: 64,
            graph_signature: String::new(),
            production_safe: true,
        };
        let mut baseline = AnnRecallLatencyBaseline::synthetic_v1();
        baseline.vector_count = 11;
        baseline.min_graph_edges = 21;

        let failures = compare_ann_fixture_baseline(&baseline, &observed);

        assert!(failures.iter().any(|value| value.contains("vector_count")));
        assert!(failures.iter().any(|value| value.contains("graph_edges")));
    }
}
