use super::*;

fn baseline() -> AnnMetricMatrixBaseline {
    AnnMetricMatrixBaseline {
        baseline_id: "metric-test".to_owned(),
        fixture_id: "metric-fixture".to_owned(),
        fixture_path: "unused".to_owned(),
        policy_min_recall_q16: MIN_ANN_RECALL_Q16,
        require_slo: true,
        metrics: ["dot_product", "cosine", "l2"]
            .into_iter()
            .map(|metric| AnnMetricBaseline {
                metric: metric.to_owned(),
                min_observed_recall_q16: MIN_ANN_RECALL_Q16,
                min_mean_recall_q16: MIN_ANN_RECALL_Q16,
                min_graph_nodes: 4,
                min_graph_edges: 4,
                min_upper_layers: 0,
                min_upper_graph_edges: 0,
                max_p95_latency_nanos: 500_000_000,
                max_p99_latency_nanos: 500_000_000,
                max_max_latency_nanos: 500_000_000,
                require_production_safe: true,
            })
            .collect(),
    }
}

#[test]
fn metric_matrix_evaluates_all_configured_metrics() {
    let fixture = r#"
{"kind":"vector","candidate":1,"vector":[100,0,0,0]}
{"kind":"vector","candidate":2,"vector":[96,4,0,0]}
{"kind":"vector","candidate":3,"vector":[0,100,0,0]}
{"kind":"vector","candidate":4,"vector":[0,96,4,0]}
{"kind":"query","name":"axis-a","vector":[100,0,0,0],"limit":2}
{"kind":"query","name":"axis-b","vector":[0,100,0,0],"limit":2}
"#;

    let report = evaluate_ann_metric_matrix(&baseline(), fixture).unwrap();

    assert!(report.passed, "{:?}", report.failures);
    assert_eq!(report.metrics.len(), 3);
    assert!(report.metrics.iter().all(|metric| metric.production_safe));
}

#[test]
fn metric_matrix_rejects_unknown_metric() {
    let mut baseline = baseline();
    baseline.metrics[0].metric = "unknown".to_owned();
    let fixture = r#"
{"kind":"vector","candidate":1,"vector":[100,0,0,0]}
{"kind":"query","name":"axis-a","vector":[100,0,0,0],"limit":1}
"#;

    let error = evaluate_ann_metric_matrix(&baseline, fixture).unwrap_err();

    assert!(error.to_string().contains("unknown metric"));
}
