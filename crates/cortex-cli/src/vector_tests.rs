use super::run;

#[test]
fn search_vector_command_returns_scope_filtered_results() {
    let path = unique_path("cortexdb-cli-vector-search");
    let path_arg = path.to_string_lossy().into_owned();
    run(vec![
        "cortexdb".to_owned(),
        "put".to_owned(),
        path_arg.clone(),
        "1".to_owned(),
        "scope=project:investments\nstatus=ready\nvector=5,0\nalpha".to_owned(),
    ])
    .unwrap();
    run(vec![
        "cortexdb".to_owned(),
        "put".to_owned(),
        path_arg.clone(),
        "2".to_owned(),
        "scope=tenant:private\nstatus=ready\nvector=9,0\nhidden".to_owned(),
    ])
    .unwrap();

    let output = run(vec![
        "cortexdb".to_owned(),
        "search-vector".to_owned(),
        path_arg.clone(),
        "project:investments".to_owned(),
        "2,0".to_owned(),
    ])
    .unwrap();
    assert!(output.contains("results=1"));
    assert!(output.contains("cell_id=1"));
    assert!(output.contains("vector_score="));
    assert!(!output.contains("cell_id=2"));

    let _ = std::fs::remove_dir_all(path);
}

#[test]
fn search_vector_command_respects_ann_policy_flags() {
    let path = unique_path("cortexdb-cli-vector-policy");
    let path_arg = path.to_string_lossy().into_owned();
    run(vec![
        "cortexdb".to_owned(),
        "put".to_owned(),
        path_arg.clone(),
        "1".to_owned(),
        "scope=project:investments\nstatus=ready\nvector=5,0\nalpha".to_owned(),
    ])
    .unwrap();
    run(vec![
        "cortexdb".to_owned(),
        "put".to_owned(),
        path_arg.clone(),
        "2".to_owned(),
        "scope=project:investments\nstatus=ready\nvector=9,1\nbeta".to_owned(),
    ])
    .unwrap();
    run(vec![
        "cortexdb".to_owned(),
        "flush".to_owned(),
        path_arg.clone(),
    ])
    .unwrap();

    let default_output = run(vec![
        "cortexdb".to_owned(),
        "search-vector".to_owned(),
        path_arg.clone(),
        "project:investments".to_owned(),
        "5,0".to_owned(),
    ])
    .unwrap();
    assert!(default_output.contains("ann_path="));
    assert!(default_output.contains("min_recall_q16=49151"));

    let strict_output = run(vec![
        "cortexdb".to_owned(),
        "search-vector".to_owned(),
        "--fallback".to_owned(),
        "false".to_owned(),
        "--fallback-scan-cap".to_owned(),
        "0".to_owned(),
        "--min-recall".to_owned(),
        "50%".to_owned(),
        "--require-slo".to_owned(),
        "--no-fallback-rollout".to_owned(),
        "--no-fallback-min-recall".to_owned(),
        "50%".to_owned(),
        path_arg.clone(),
        "project:investments".to_owned(),
        "5,0".to_owned(),
    ])
    .unwrap();
    assert!(strict_output.contains("ann_path="));
    assert!(strict_output.contains("min_recall_q16=32767"));
    assert!(strict_output.contains("no_fallback_allowed=true"));

    let _ = std::fs::remove_dir_all(path);
}

#[test]
fn search_vector_eval_command_reports_recall_after_flush() {
    let path = unique_path("cortexdb-cli-vector-eval");
    let path_arg = path.to_string_lossy().into_owned();
    run(vec![
        "cortexdb".to_owned(),
        "put".to_owned(),
        path_arg.clone(),
        "1".to_owned(),
        "scope=project:investments\nstatus=ready\nvector=10,0\nalpha".to_owned(),
    ])
    .unwrap();
    run(vec![
        "cortexdb".to_owned(),
        "put".to_owned(),
        path_arg.clone(),
        "2".to_owned(),
        "scope=project:investments\nstatus=ready\nvector=0,10\nbeta".to_owned(),
    ])
    .unwrap();

    let unavailable = run(vec![
        "cortexdb".to_owned(),
        "search-vector-eval".to_owned(),
        path_arg.clone(),
        "project:investments".to_owned(),
        "0,10".to_owned(),
    ])
    .unwrap();
    assert!(unavailable.contains("available=false"));

    run(vec![
        "cortexdb".to_owned(),
        "flush".to_owned(),
        path_arg.clone(),
    ])
    .unwrap();

    let output = run(vec![
        "cortexdb".to_owned(),
        "search-vector-eval".to_owned(),
        path_arg.clone(),
        "project:investments".to_owned(),
        "0,10".to_owned(),
    ])
    .unwrap();
    assert!(output.contains("available=true"));
    assert!(output.contains("recall_q16=65535"));
    assert!(output.contains("exact_top_k=[2, 1]"));

    let json = run(vec![
        "cortexdb".to_owned(),
        "--json".to_owned(),
        "search-vector-eval".to_owned(),
        path_arg.clone(),
        "project:investments".to_owned(),
        "0,10".to_owned(),
    ])
    .unwrap();
    let response: serde_json::Value = serde_json::from_str(&json)
        .unwrap_or_else(|error| panic!("invalid json response: {error}"));
    assert_eq!(response["available"], true);
    assert_eq!(response["recall_q16"], 65535);
    assert!(response["ann_report"]["hnsw_max_neighbors"]
        .as_u64()
        .is_some());
    assert!(response["ann_report"]["hnsw_layer_count"]
        .as_u64()
        .is_some());
    assert!(response["ann_report"]["upper_graph_edges"]
        .as_u64()
        .is_some());
    assert!(response["ann_report"]["hnsw_ef_search"].as_u64().is_some());
    assert!(response["ann_report"]["hnsw_ef_construction"]
        .as_u64()
        .is_some());

    let _ = std::fs::remove_dir_all(path);
}

#[test]
fn search_vector_eval_command_applies_min_recall_policy() {
    let path = unique_path("cortexdb-cli-vector-eval-policy");
    let path_arg = path.to_string_lossy().into_owned();
    run(vec![
        "cortexdb".to_owned(),
        "put".to_owned(),
        path_arg.clone(),
        "1".to_owned(),
        "scope=project:investments\nstatus=ready\nvector=10,0\nalpha".to_owned(),
    ])
    .unwrap();
    run(vec![
        "cortexdb".to_owned(),
        "put".to_owned(),
        path_arg.clone(),
        "2".to_owned(),
        "scope=project:investments\nstatus=ready\nvector=0,10\nbeta".to_owned(),
    ])
    .unwrap();
    run(vec![
        "cortexdb".to_owned(),
        "flush".to_owned(),
        path_arg.clone(),
    ])
    .unwrap();

    let json = run(vec![
        "cortexdb".to_owned(),
        "--json".to_owned(),
        "search-vector-eval".to_owned(),
        "--fallback".to_owned(),
        "false".to_owned(),
        "--min-recall".to_owned(),
        "50%".to_owned(),
        "--require-slo".to_owned(),
        "--no-fallback-rollout".to_owned(),
        "--no-fallback-min-recall".to_owned(),
        "50%".to_owned(),
        path_arg.clone(),
        "project:investments".to_owned(),
        "0,10".to_owned(),
    ])
    .unwrap();
    let response: serde_json::Value = serde_json::from_str(&json)
        .unwrap_or_else(|error| panic!("invalid json response: {error}"));
    assert_eq!(response["ann_report"]["min_recall_q16"], 32767);
    assert!(response["ann_report"]["slo_violations"].is_array());
    assert_eq!(response["no_fallback_decision"]["allowed"], true);
    assert!(response["no_fallback_decision"]["reasons"]
        .as_array()
        .unwrap()
        .is_empty());

    let _ = std::fs::remove_dir_all(path);
}

fn unique_path(prefix: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "{prefix}-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ))
}
