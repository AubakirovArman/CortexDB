use serde::{Deserialize, Serialize};

use crate::error::EngineResult;

use super::ann::AnnSearchPolicy;
use super::ann_report::{synthetic_ann_recall_latency_report, AnnRecallLatencyReport};

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct AnnDriftBaseline {
    pub baseline_id: String,
    pub corpus: String,
    pub vector_count: usize,
    pub dimension: usize,
    pub query_count: usize,
    pub limit: usize,
    pub policy_min_recall_q16: u16,
    pub require_slo: bool,
    pub reference_min_observed_recall_q16: u16,
    pub reference_mean_recall_q16: u16,
    pub reference_graph_nodes: usize,
    pub reference_graph_edges: usize,
    pub reference_upper_layers: usize,
    pub reference_upper_graph_edges: usize,
    #[serde(default)]
    pub reference_hnsw_max_neighbors: Option<usize>,
    #[serde(default)]
    pub reference_hnsw_ef_search: Option<usize>,
    #[serde(default)]
    pub reference_hnsw_layer_count: Option<usize>,
    pub reference_p95_latency_nanos: u128,
    #[serde(default)]
    pub reference_p99_latency_nanos: Option<u128>,
    pub reference_max_latency_nanos: u128,
    pub allowed_recall_drop_q16: u16,
    pub max_latency_regression_percent: u32,
    pub max_edge_loss_percent: u32,
    pub require_production_safe: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct AnnDriftReport {
    pub baseline_id: String,
    pub passed: bool,
    pub failures: Vec<String>,
    pub observed: AnnRecallLatencyReport,
    pub min_recall_delta_q16: i32,
    pub mean_recall_delta_q16: i32,
    pub p95_latency_delta_nanos: i128,
    pub p99_latency_delta_nanos: i128,
    pub max_latency_delta_nanos: i128,
}

impl AnnDriftReport {
    pub fn as_json(&self) -> String {
        serde_json::to_string(self)
            .unwrap_or_else(|_| "{\"error\":\"ann_drift_report_serialization_failed\"}".to_owned())
    }
}

pub fn evaluate_ann_drift_baseline(baseline: &AnnDriftBaseline) -> EngineResult<AnnDriftReport> {
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
    let failures = compare_ann_drift_baseline(baseline, &observed);
    Ok(AnnDriftReport {
        baseline_id: baseline.baseline_id.clone(),
        passed: failures.is_empty(),
        min_recall_delta_q16: i32::from(observed.min_observed_recall_q16)
            - i32::from(baseline.reference_min_observed_recall_q16),
        mean_recall_delta_q16: i32::from(observed.mean_recall_q16)
            - i32::from(baseline.reference_mean_recall_q16),
        p95_latency_delta_nanos: saturating_delta(
            observed.p95_latency_nanos,
            baseline.reference_p95_latency_nanos,
        ),
        p99_latency_delta_nanos: saturating_delta(
            observed.p99_latency_nanos,
            baseline.reference_p99_latency_nanos(),
        ),
        max_latency_delta_nanos: saturating_delta(
            observed.max_latency_nanos,
            baseline.reference_max_latency_nanos,
        ),
        failures,
        observed,
    })
}

pub fn compare_ann_drift_baseline(
    baseline: &AnnDriftBaseline,
    observed: &AnnRecallLatencyReport,
) -> Vec<String> {
    let mut failures = Vec::new();
    check_identity(baseline, observed, &mut failures);
    check_recall(
        &mut failures,
        "min_observed_recall_q16",
        observed.min_observed_recall_q16,
        baseline.reference_min_observed_recall_q16,
        baseline.allowed_recall_drop_q16,
    );
    check_recall(
        &mut failures,
        "mean_recall_q16",
        observed.mean_recall_q16,
        baseline.reference_mean_recall_q16,
        baseline.allowed_recall_drop_q16,
    );
    check_min(
        &mut failures,
        "graph_nodes",
        observed.graph_nodes,
        min_allowed_usize(
            baseline.reference_graph_nodes,
            baseline.max_edge_loss_percent,
        ),
    );
    check_min(
        &mut failures,
        "graph_edges",
        observed.graph_edges,
        min_allowed_usize(
            baseline.reference_graph_edges,
            baseline.max_edge_loss_percent,
        ),
    );
    check_min(
        &mut failures,
        "upper_layers",
        observed.upper_layers,
        min_allowed_usize(
            baseline.reference_upper_layers,
            baseline.max_edge_loss_percent,
        ),
    );
    check_min(
        &mut failures,
        "upper_graph_edges",
        observed.upper_graph_edges,
        min_allowed_usize(
            baseline.reference_upper_graph_edges,
            baseline.max_edge_loss_percent,
        ),
    );
    if let Some(reference_hnsw_max_neighbors) = baseline.reference_hnsw_max_neighbors {
        check_eq(
            &mut failures,
            "reference_hnsw_max_neighbors",
            reference_hnsw_max_neighbors,
            observed.hnsw_max_neighbors,
        );
    }
    if let Some(reference_hnsw_ef_search) = baseline.reference_hnsw_ef_search {
        check_eq(
            &mut failures,
            "reference_hnsw_ef_search",
            reference_hnsw_ef_search,
            observed.hnsw_ef_search,
        );
    }
    if let Some(reference_hnsw_layer_count) = baseline.reference_hnsw_layer_count {
        check_eq(
            &mut failures,
            "reference_hnsw_layer_count",
            reference_hnsw_layer_count,
            observed.hnsw_layer_count,
        );
    }
    check_latency(
        &mut failures,
        "p95_latency_nanos",
        observed.p95_latency_nanos,
        baseline.reference_p95_latency_nanos,
        baseline.max_latency_regression_percent,
    );
    check_latency(
        &mut failures,
        "p99_latency_nanos",
        observed.p99_latency_nanos,
        baseline.reference_p99_latency_nanos(),
        baseline.max_latency_regression_percent,
    );
    check_latency(
        &mut failures,
        "max_latency_nanos",
        observed.max_latency_nanos,
        baseline.reference_max_latency_nanos,
        baseline.max_latency_regression_percent,
    );
    if baseline.require_production_safe && !observed.production_safe {
        failures.push("production_safe: expected true, observed false".to_owned());
    }
    failures
}

impl AnnDriftBaseline {
    pub fn reference_p99_latency_nanos(&self) -> u128 {
        self.reference_p99_latency_nanos
            .unwrap_or(self.reference_max_latency_nanos)
    }
}

fn check_identity(
    baseline: &AnnDriftBaseline,
    observed: &AnnRecallLatencyReport,
    failures: &mut Vec<String>,
) {
    if observed.corpus != baseline.corpus {
        failures.push(format!(
            "corpus: expected {}, observed {}",
            baseline.corpus, observed.corpus
        ));
    }
    check_eq(
        failures,
        "vector_count",
        baseline.vector_count,
        observed.vector_count,
    );
    check_eq(
        failures,
        "dimension",
        baseline.dimension,
        observed.dimension,
    );
    check_eq(
        failures,
        "query_count",
        baseline.query_count,
        observed.query_count,
    );
    check_eq(failures, "limit", baseline.limit, observed.limit);
}

fn check_eq<T>(failures: &mut Vec<String>, field: &str, expected: T, observed: T)
where
    T: std::fmt::Display + PartialEq,
{
    if observed != expected {
        failures.push(format!("{field}: expected {expected}, observed {observed}"));
    }
}

fn check_recall(
    failures: &mut Vec<String>,
    field: &str,
    observed: u16,
    reference: u16,
    allowed_drop: u16,
) {
    let minimum = reference.saturating_sub(allowed_drop);
    if observed < minimum {
        failures.push(format!(
            "{field}: expected >= {minimum} from reference {reference}, observed {observed}"
        ));
    }
}

fn check_min(failures: &mut Vec<String>, field: &str, observed: usize, minimum: usize) {
    if observed < minimum {
        failures.push(format!(
            "{field}: expected >= {minimum}, observed {observed}"
        ));
    }
}

fn check_latency(
    failures: &mut Vec<String>,
    field: &str,
    observed: u128,
    reference: u128,
    regression_percent: u32,
) {
    let maximum = reference + (reference * u128::from(regression_percent) / 100);
    if observed > maximum {
        failures.push(format!(
            "{field}: expected <= {maximum} from reference {reference}, observed {observed}"
        ));
    }
}

fn min_allowed_usize(reference: usize, allowed_loss_percent: u32) -> usize {
    let loss = (reference as u128 * u128::from(allowed_loss_percent) / 100) as usize;
    reference.saturating_sub(loss)
}

fn saturating_delta(observed: u128, reference: u128) -> i128 {
    if observed >= reference {
        observed.saturating_sub(reference).min(i128::MAX as u128) as i128
    } else {
        -(reference.saturating_sub(observed).min(i128::MAX as u128) as i128)
    }
}

#[cfg(test)]
mod tests;
