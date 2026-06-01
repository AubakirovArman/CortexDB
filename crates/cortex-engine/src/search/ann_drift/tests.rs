use super::*;
use crate::search::ann_report::SYNTHETIC_ANN_CORPUS_V1;

fn baseline() -> AnnDriftBaseline {
    serde_json::from_str(include_str!("../../../fixtures/ann_drift_baseline_v1.json")).unwrap()
}

fn observed_from_baseline(baseline: &AnnDriftBaseline) -> AnnRecallLatencyReport {
    AnnRecallLatencyReport {
        corpus: SYNTHETIC_ANN_CORPUS_V1,
        vector_count: baseline.vector_count,
        dimension: baseline.dimension,
        query_count: baseline.query_count,
        limit: baseline.limit,
        graph_nodes: baseline.reference_graph_nodes,
        graph_edges: baseline.reference_graph_edges,
        upper_layers: baseline.reference_upper_layers,
        upper_graph_edges: baseline.reference_upper_graph_edges,
        hnsw_max_neighbors: baseline.reference_hnsw_max_neighbors.unwrap_or(8),
        hnsw_ef_search: baseline.reference_hnsw_ef_search.unwrap_or(64),
        hnsw_layer_count: baseline.reference_hnsw_layer_count.unwrap_or(4),
        hnsw_ef_construction: baseline.reference_hnsw_ef_search.unwrap_or(64),
        graph_signature: String::new(),
        min_recall_q16: baseline.policy_min_recall_q16,
        min_observed_recall_q16: baseline.reference_min_observed_recall_q16,
        mean_recall_q16: baseline.reference_mean_recall_q16,
        p50_latency_nanos: baseline.reference_p95_latency_nanos,
        p95_latency_nanos: baseline.reference_p95_latency_nanos,
        p99_latency_nanos: baseline.reference_p99_latency_nanos(),
        max_latency_nanos: baseline.reference_max_latency_nanos,
        production_safe: true,
    }
}

#[test]
fn drift_baseline_accepts_matching_report() {
    let baseline = baseline();
    let observed = observed_from_baseline(&baseline);

    let failures = compare_ann_drift_baseline(&baseline, &observed);

    assert!(failures.is_empty(), "{failures:?}");
}

#[test]
fn drift_baseline_rejects_recall_and_latency_regression() {
    let baseline = baseline();
    let mut observed = observed_from_baseline(&baseline);
    observed.min_observed_recall_q16 = 1;
    observed.p95_latency_nanos = baseline.reference_p95_latency_nanos * 10;
    observed.p99_latency_nanos = baseline.reference_p99_latency_nanos() * 10;

    let failures = compare_ann_drift_baseline(&baseline, &observed);

    assert!(failures
        .iter()
        .any(|value| value.contains("min_observed_recall_q16")));
    assert!(failures
        .iter()
        .any(|value| value.contains("p95_latency_nanos")));
    assert!(failures
        .iter()
        .any(|value| value.contains("p99_latency_nanos")));
}
