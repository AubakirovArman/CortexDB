use super::helpers::*;

#[test]
fn aql_command_returns_retrieved_cells() {
    let path = unique_path("cortexdb-cli-aql");
    let path_arg = path.to_string_lossy().into_owned();
    run(vec![
        "cortexdb".to_owned(),
        "put".to_owned(),
        path_arg.clone(),
        "1".to_owned(),
        "scope=project:investments\nstatus=ready\nalpha budget".to_owned(),
    ])
    .unwrap();

    let output = run(vec![
        "cortexdb".to_owned(),
        "aql".to_owned(),
        path_arg.clone(),
        "project:investments".to_owned(),
        r#"RETRIEVE CONTEXT FOR TASK "budget" IN BRAIN investment_projects WHERE space = project:investments AND status = "ready" LIMIT 10 CANDIDATES;"#.to_owned(),
    ])
    .unwrap();
    assert!(output.contains("cells=1"));
    assert!(output.contains("cell_id=1"));
    assert!(output.contains("alpha budget"));

    let _ = std::fs::remove_dir_all(path);
}

#[test]
fn aql_command_explain_reports_plan_filters_counts_and_mode() {
    let path = unique_path("cortexdb-cli-aql-explain");
    let path_arg = path.to_string_lossy().into_owned();
    run(vec![
        "cortexdb".to_owned(),
        "put".to_owned(),
        path_arg.clone(),
        "1".to_owned(),
        "scope=project:investments\nstatus=ready\nalpha budget".to_owned(),
    ])
    .unwrap();

    let statement = r#"EXPLAIN RETRIEVE CONTEXT FOR TASK "budget" IN BRAIN investment_projects WHERE space = project:investments AND status = "ready" LIMIT 10 CANDIDATES;"#;
    let output = run(vec![
        "cortexdb".to_owned(),
        "aql".to_owned(),
        path_arg.clone(),
        "project:investments".to_owned(),
        statement.to_owned(),
    ])
    .unwrap();
    assert!(output.contains("aql_explain task=budget"));
    assert!(output.contains("mode=Balanced"));
    assert!(output.contains("cost_model selected_path=bitmap-first"));
    assert!(output.contains("after_bitmap=1"));
    assert!(output.contains("filters=policy=agent_allowed"));
    assert!(output.contains("BitmapProgram(max_stack_depth="));

    let json = run(vec![
        "cortexdb".to_owned(),
        "--json".to_owned(),
        "aql".to_owned(),
        path_arg.clone(),
        "project:investments".to_owned(),
        statement.to_owned(),
    ])
    .unwrap();
    assert!(json.contains(r#""cells":[]"#));
    assert!(json.contains(r#""selected_mode":"balanced""#));
    assert!(json.contains(r#""cost_model":{"selected_path":"bitmap-first""#));
    assert!(json.contains(r#""after_bitmap":1"#));

    let _ = std::fs::remove_dir_all(path);
}

#[test]
fn search_command_returns_scope_filtered_results() {
    let path = unique_path("cortexdb-cli-search");
    let path_arg = path.to_string_lossy().into_owned();
    run(vec![
        "cortexdb".to_owned(),
        "put".to_owned(),
        path_arg.clone(),
        "1".to_owned(),
        "scope=project:investments\nstatus=ready\nalpha budget".to_owned(),
    ])
    .unwrap();
    run(vec![
        "cortexdb".to_owned(),
        "put".to_owned(),
        path_arg.clone(),
        "2".to_owned(),
        "scope=tenant:private\nstatus=ready\nhidden budget".to_owned(),
    ])
    .unwrap();

    let output = run(vec![
        "cortexdb".to_owned(),
        "search".to_owned(),
        path_arg.clone(),
        "project:investments".to_owned(),
        "budget".to_owned(),
    ])
    .unwrap();
    assert!(output.contains("results=1"));
    assert!(output.contains("cell_id=1"));
    assert!(!output.contains("cell_id=2"));

    let _ = std::fs::remove_dir_all(path);
}

#[test]
fn search_command_auto_mode_reports_routing_json() {
    let path = unique_path("cortexdb-cli-search-routing");
    let path_arg = path.to_string_lossy().into_owned();
    run(vec![
        "cortexdb".to_owned(),
        "put".to_owned(),
        path_arg.clone(),
        "1".to_owned(),
        "scope=project:investments\nstatus=ready\nvector=5,0\nalpha budget".to_owned(),
    ])
    .unwrap();

    let output = run(vec![
        "cortexdb".to_owned(),
        "--json".to_owned(),
        "search".to_owned(),
        path_arg.clone(),
        "project:investments".to_owned(),
        "budget".to_owned(),
        "--mode".to_owned(),
        "auto".to_owned(),
        "--vector".to_owned(),
        "5,0".to_owned(),
    ])
    .unwrap();
    assert!(output.contains(r#""search_mode":"hybrid""#));
    assert!(output.contains(r#""selected_strategy":"hybrid""#));
    assert!(output.contains(r#""reason":"auto_text_and_vector_available""#));

    let _ = std::fs::remove_dir_all(path);
}

#[test]
fn search_command_hybrid_uses_persisted_indexes_after_flush() {
    let path = unique_path("cortexdb-cli-search-hybrid-persisted");
    let path_arg = path.to_string_lossy().into_owned();
    run(vec![
        "cortexdb".to_owned(),
        "put".to_owned(),
        path_arg.clone(),
        "1".to_owned(),
        "scope=project:investments\nstatus=ready\nvector=1,0,0\n\nbudget investment".to_owned(),
    ])
    .unwrap();
    run(vec![
        "cortexdb".to_owned(),
        "flush".to_owned(),
        path_arg.clone(),
    ])
    .unwrap();

    let output = run(vec![
        "cortexdb".to_owned(),
        "--json".to_owned(),
        "search".to_owned(),
        path_arg.clone(),
        "project:investments".to_owned(),
        "budget".to_owned(),
        "--mode".to_owned(),
        "hybrid".to_owned(),
        "--vector".to_owned(),
        "1,0,0".to_owned(),
    ])
    .unwrap();

    assert!(output.contains(r#""search_mode":"hybrid""#));
    assert!(output.contains(r#""cell_id":1"#));
    assert!(output.contains(r#""lexical_score":"#));
    assert!(output.contains(r#""vector_score":"#));

    let _ = std::fs::remove_dir_all(path);
}

#[test]
fn search_explain_command_reports_contribution_details() {
    let path = unique_path("cortexdb-cli-search-explain");
    let path_arg = path.to_string_lossy().into_owned();
    run(vec![
        "cortexdb".to_owned(),
        "put".to_owned(),
        path_arg.clone(),
        "1".to_owned(),
        "scope=project:investments\nstatus=ready\nvector=5,0\nalpha budget budget".to_owned(),
    ])
    .unwrap();

    let keyword = run(vec![
        "cortexdb".to_owned(),
        "search-explain".to_owned(),
        path_arg.clone(),
        "project:investments".to_owned(),
        "budget".to_owned(),
    ])
    .unwrap();
    assert!(keyword.contains("query_terms_count=1"));
    assert!(keyword.contains("rank=1"));
    assert!(keyword.contains("lexical_q16=65535"));
    assert!(keyword.contains("fusion=false"));

    let hybrid = run(vec![
        "cortexdb".to_owned(),
        "search-explain".to_owned(),
        path_arg.clone(),
        "project:investments".to_owned(),
        "budget".to_owned(),
        "--mode".to_owned(),
        "hybrid".to_owned(),
        "--vector".to_owned(),
        "5,0".to_owned(),
    ])
    .unwrap();
    assert!(hybrid.contains("rank=1"));
    assert!(hybrid.contains("fusion=true"));

    let _ = std::fs::remove_dir_all(path);
}
