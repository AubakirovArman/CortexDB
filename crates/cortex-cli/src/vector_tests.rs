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
    assert!(json.contains(r#""available":true"#));
    assert!(json.contains(r#""recall_q16":65535"#));

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
