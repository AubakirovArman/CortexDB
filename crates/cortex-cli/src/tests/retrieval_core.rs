use super::helpers::*;

#[test]
fn vector_rebuild_command_repairs_corrupt_ann_files() {
    let path = unique_path("cortexdb-cli-vector-rebuild");
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
        "--experimental-hnsw".to_owned(),
    ])
    .unwrap();

    let graph_path = path.join("segments/segment-1.ach");
    let mut bytes = std::fs::read(&graph_path).unwrap();
    bytes.truncate(bytes.len().saturating_sub(4));
    std::fs::write(&graph_path, bytes).unwrap();

    let output = run(vec![
        "cortexdb".to_owned(),
        "vector".to_owned(),
        "rebuild".to_owned(),
        path_arg.clone(),
        "--experimental-hnsw".to_owned(),
    ])
    .unwrap();
    assert!(output.contains("vector_rebuild"));
    assert!(output.contains("vector_indexes_rebuilt=1"));
    assert!(output.contains("hnsw_graphs_rebuilt=1"));

    let json = run(vec![
        "cortexdb".to_owned(),
        "--json".to_owned(),
        "vector".to_owned(),
        "rebuild".to_owned(),
        path_arg.clone(),
        "--experimental-hnsw".to_owned(),
    ])
    .unwrap();
    assert!(json.contains(r#""vector_indexes_rebuilt":1"#));
    assert!(json.contains(r#""hnsw_graphs_rebuilt":1"#));
    assert!(json.contains(r#""hnsw_enabled":true"#));

    let _ = std::fs::remove_dir_all(path);
}

#[test]
fn stats_and_validate_commands_work() {
    let path = unique_path("cortexdb-cli-stats");
    let path_arg = path.to_string_lossy().into_owned();
    run(vec![
        "cortexdb".to_owned(),
        "put".to_owned(),
        path_arg.clone(),
        "1".to_owned(),
        "hello".to_owned(),
    ])
    .unwrap();

    let stats = run(vec![
        "cortexdb".to_owned(),
        "stats".to_owned(),
        path_arg.clone(),
    ])
    .unwrap();
    assert!(stats.contains("current_seq=1"));
    assert!(stats.contains("logical_payload_bytes="));
    assert!(stats.contains("space_amplification_q16="));
    assert!(stats.contains("write_amplification_q16="));
    assert!(stats.contains("compaction_pressure_q16=0"));
    assert!(stats.contains("wal_writer_records=0"));

    let validation = run(vec![
        "cortexdb".to_owned(),
        "validate".to_owned(),
        path_arg.clone(),
    ])
    .unwrap();
    assert!(validation.starts_with("ok "));

    let ann_val = run(vec![
        "cortexdb".to_owned(),
        "ann-validate".to_owned(),
        path_arg.clone(),
    ])
    .unwrap();
    assert!(ann_val.contains("ok vector_indexes_checked="));

    let _ = std::fs::remove_dir_all(path);
}

#[test]
fn context_command_returns_pack_summary() {
    let path = unique_path("cortexdb-cli-context");
    let path_arg = path.to_string_lossy().into_owned();
    run(vec![
        "cortexdb".to_owned(),
        "put".to_owned(),
        path_arg.clone(),
        "1".to_owned(),
        "scope=project:investments\nstatus=ready\nsource=doc-a\nalpha budget".to_owned(),
    ])
    .unwrap();

    let output = run(vec![
        "cortexdb".to_owned(),
        "context".to_owned(),
        path_arg.clone(),
        "project:investments".to_owned(),
        r#"RETRIEVE CONTEXT FOR TASK "budget" IN BRAIN investment_projects WHERE space = project:investments AND status = "ready" LIMIT 10 CANDIDATES;"#.to_owned(),
    ])
    .unwrap();
    assert!(output.contains("cells=1"));
    assert!(output.contains("citation=doc-a"));

    let prompt = run(vec![
        "cortexdb".to_owned(),
        "context".to_owned(),
        path_arg.clone(),
        "project:investments".to_owned(),
        r#"RETRIEVE CONTEXT FOR TASK "budget" IN BRAIN investment_projects WHERE space = project:investments AND status = "ready" LIMIT 10 CANDIDATES;"#.to_owned(),
        "--format".to_owned(),
        "prompt".to_owned(),
    ])
    .unwrap();
    assert!(prompt.contains("CortexDB ContextPack v1"));
    assert!(prompt.contains("Use only the context cells below."));

    let markdown = run(vec![
        "cortexdb".to_owned(),
        "context".to_owned(),
        path_arg.clone(),
        "project:investments".to_owned(),
        r#"RETRIEVE CONTEXT FOR TASK "budget" IN BRAIN investment_projects WHERE space = project:investments AND status = "ready" LIMIT 10 CANDIDATES;"#.to_owned(),
        "--format".to_owned(),
        "markdown".to_owned(),
    ])
    .unwrap();
    assert!(markdown.contains("# CortexDB ContextPack"));
    assert!(markdown.contains("### Cell 1"));

    let _ = std::fs::remove_dir_all(path);
}

#[test]
fn remember_and_verify_commands_work() {
    let path = unique_path("cortexdb-cli-memory");
    let path_arg = path.to_string_lossy().into_owned();
    let remember = run(vec![
        "cortexdb".to_owned(),
        "remember".to_owned(),
        path_arg.clone(),
        "project:investments".to_owned(),
        r#"REMEMBER "ABC budget approved" IN SCOPE project:investments AS TYPE decision TTL 60 SECONDS;"#.to_owned(),
    ])
    .unwrap();
    assert!(remember.contains("seq=1"));
    assert!(remember.contains("ttl_seconds=60"));

    let verify = run(vec![
        "cortexdb".to_owned(),
        "verify".to_owned(),
        path_arg.clone(),
        "project:investments".to_owned(),
        r#"VERIFY FACT "ABC budget approved" IN BRAIN investment_projects;"#.to_owned(),
    ])
    .unwrap();
    assert!(verify.contains("status=supported"));
    assert!(verify.contains("evidence=1"));

    let _ = std::fs::remove_dir_all(path);
}
