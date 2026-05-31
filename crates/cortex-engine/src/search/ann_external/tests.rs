use super::*;
use crate::search::MIN_ANN_RECALL_Q16;

fn baseline() -> AnnExternalFixtureBaseline {
    AnnExternalFixtureBaseline {
        baseline_id: "test".to_owned(),
        fixture_id: "test-fixture".to_owned(),
        fixture_path: "unused".to_owned(),
        policy_min_recall_q16: MIN_ANN_RECALL_Q16,
        require_slo: true,
        min_observed_recall_q16: MIN_ANN_RECALL_Q16,
        min_mean_recall_q16: MIN_ANN_RECALL_Q16,
        min_graph_nodes: 4,
        min_graph_edges: 4,
        min_upper_layers: 0,
        min_upper_graph_edges: 0,
        max_p95_latency_nanos: 500_000_000,
        max_max_latency_nanos: 500_000_000,
        require_production_safe: false,
    }
}

#[test]
fn external_fixture_evaluates_jsonl() {
    let fixture = r#"
{"kind":"vector","candidate":1,"vector":[100,0,0,0]}
{"kind":"vector","candidate":2,"vector":[96,4,0,0]}
{"kind":"vector","candidate":3,"vector":[0,100,0,0]}
{"kind":"vector","candidate":4,"vector":[0,96,4,0]}
{"kind":"query","name":"axis-a","vector":[100,0,0,0],"limit":2}
{"kind":"query","name":"axis-b","vector":[0,100,0,0],"limit":2}
"#;

    let report = evaluate_ann_external_fixture(&baseline(), fixture).unwrap();

    assert!(report.passed, "{:?}", report.failures);
    assert_eq!(report.vector_count, 4);
    assert_eq!(report.query_count, 2);
    assert_eq!(report.dimension, 4);
}

#[test]
fn external_fixture_file_baseline_is_production_safe() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let fixture_path = format!("{manifest_dir}/fixtures/ann_external_fixture_v1.jsonl");
    let baseline_path = format!("{manifest_dir}/fixtures/ann_external_baseline_v1.json");
    let fixture = std::fs::read_to_string(fixture_path).unwrap();
    let baseline: AnnExternalFixtureBaseline =
        serde_json::from_str(&std::fs::read_to_string(baseline_path).unwrap()).unwrap();
    let report = evaluate_ann_external_fixture(&baseline, &fixture).unwrap();

    assert!(report.passed, "{:?}", report.failures);
    assert!(report.production_safe);
}

#[test]
fn external_fixture_rejects_dimension_mismatch() {
    let fixture = r#"
{"kind":"vector","candidate":1,"vector":[100,0,0,0]}
{"kind":"query","name":"bad","vector":[100,0],"limit":2}
"#;

    let error = evaluate_ann_external_fixture(&baseline(), fixture).unwrap_err();

    assert!(error.to_string().contains("dimension"));
}
