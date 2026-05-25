use super::run;

#[test]
fn usage_is_reported_for_missing_args() {
    assert!(run(vec!["cortexdb".to_owned()])
        .unwrap_err()
        .contains("usage:"));
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

    let validation = run(vec!["cortexdb".to_owned(), "validate".to_owned(), path_arg]).unwrap();
    assert!(validation.starts_with("ok "));

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

    let _ = std::fs::remove_dir_all(path);
}

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
fn repair_command_reports_best_effort_cleanup() {
    let path = unique_path("cortexdb-cli-repair");
    let path_arg = path.to_string_lossy().into_owned();
    std::fs::create_dir_all(&path).unwrap();
    std::fs::write(path.join("db.aclog.tmp"), b"bad").unwrap();

    let output = run(vec![
        "cortexdb".to_owned(),
        "repair".to_owned(),
        path_arg.clone(),
    ])
    .unwrap();
    assert!(output.contains("orphan_temp_files_removed=1"));
    assert!(output.contains("wal_truncated=false"));

    let _ = std::fs::remove_dir_all(path);
}

#[test]
fn gc_retired_command_reports_removed_segments() {
    let path = unique_path("cortexdb-cli-gc-retired");
    let path_arg = path.to_string_lossy().into_owned();
    run(vec![
        "cortexdb".to_owned(),
        "put".to_owned(),
        path_arg.clone(),
        "1".to_owned(),
        "hello".to_owned(),
    ])
    .unwrap();
    run(vec![
        "cortexdb".to_owned(),
        "flush".to_owned(),
        path_arg.clone(),
    ])
    .unwrap();
    run(vec![
        "cortexdb".to_owned(),
        "compact".to_owned(),
        path_arg.clone(),
    ])
    .unwrap();

    let output = run(vec![
        "cortexdb".to_owned(),
        "gc-retired".to_owned(),
        path_arg.clone(),
    ])
    .unwrap();
    assert!(output.contains("retired_segments_removed=1"));

    let _ = std::fs::remove_dir_all(path);
}

#[test]
fn wal_validate_and_dump_report_records() {
    let path = unique_path("cortexdb-cli-wal");
    let path_arg = path.to_string_lossy().into_owned();
    run(vec![
        "cortexdb".to_owned(),
        "put".to_owned(),
        path_arg.clone(),
        "1".to_owned(),
        "hello".to_owned(),
    ])
    .unwrap();

    let validation = run(vec![
        "cortexdb".to_owned(),
        "wal-validate".to_owned(),
        path_arg.clone(),
    ])
    .unwrap();
    assert!(validation.contains("records=1"));
    assert!(validation.contains("known_sections=2"));

    let dump = run(vec![
        "cortexdb".to_owned(),
        "wal-dump".to_owned(),
        path_arg.clone(),
    ])
    .unwrap();
    assert!(dump.contains("type=PutCellBatch"));
    assert!(dump.contains("sections=2"));

    let _ = std::fs::remove_dir_all(path);
}

#[test]
fn unlock_force_removes_stale_lock() {
    let path = unique_path("cortexdb-cli-unlock");
    std::fs::create_dir_all(&path).unwrap();
    std::fs::write(path.join("db.lock"), b"stale").unwrap();
    let path_arg = path.to_string_lossy().into_owned();

    let output = run(vec![
        "cortexdb".to_owned(),
        "unlock".to_owned(),
        path_arg,
        "--force".to_owned(),
    ])
    .unwrap();
    assert_eq!(output, "stale lock removed");
    assert!(!path.join("db.lock").exists());

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
