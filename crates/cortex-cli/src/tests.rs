use super::run;

#[test]
fn usage_is_reported_for_missing_args() {
    assert!(run(vec!["cortexdb".to_owned()])
        .unwrap_err()
        .contains("Usage:"));
}

#[test]
fn help_and_version_commands_work() {
    let help = run(vec!["cortexdb".to_owned(), "--help".to_owned()]).unwrap();
    assert!(help.contains("Usage: cortexdb"));
    assert!(help.contains("ingest-json"));

    let version = run(vec!["cortexdb".to_owned(), "version".to_owned()]).unwrap();
    assert!(version.starts_with("cortexdb "));
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
fn backup_and_restore_commands_roundtrip_database() {
    let root = unique_path("cortexdb-cli-backup-root");
    let source = root.join("source");
    let backup = root.join("backup");
    let target = root.join("target");
    let source_arg = source.to_string_lossy().into_owned();
    let backup_arg = backup.to_string_lossy().into_owned();
    let target_arg = target.to_string_lossy().into_owned();

    run(vec![
        "cortexdb".to_owned(),
        "put".to_owned(),
        source_arg.clone(),
        "42".to_owned(),
        "backup payload".to_owned(),
    ])
    .unwrap();

    let backup_output = run(vec![
        "cortexdb".to_owned(),
        "backup".to_owned(),
        source_arg,
        backup_arg.clone(),
    ])
    .unwrap();
    assert!(backup_output.contains("files_copied="));

    let restore_output = run(vec![
        "cortexdb".to_owned(),
        "restore".to_owned(),
        backup_arg,
        target_arg.clone(),
    ])
    .unwrap();
    assert!(restore_output.contains("restored_wal_records_checked=1"));

    let payload = run(vec![
        "cortexdb".to_owned(),
        "get".to_owned(),
        target_arg,
        "42".to_owned(),
    ])
    .unwrap();
    assert_eq!(payload, "backup payload");

    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn backup_drill_command_restores_and_validates_copy() {
    let root = unique_path("cortexdb-cli-backup-drill-root");
    let source = root.join("source");
    let backup = root.join("backup");
    let restored = root.join("restored");
    let source_arg = source.to_string_lossy().into_owned();
    let backup_arg = backup.to_string_lossy().into_owned();
    let restored_arg = restored.to_string_lossy().into_owned();

    run(vec![
        "cortexdb".to_owned(),
        "put".to_owned(),
        source_arg.clone(),
        "43".to_owned(),
        "backup drill payload".to_owned(),
    ])
    .unwrap();

    let output = run(vec![
        "cortexdb".to_owned(),
        "backup-drill".to_owned(),
        source_arg,
        backup_arg,
        restored_arg.clone(),
    ])
    .unwrap();
    assert!(output.contains("backup_files_copied="));
    assert!(output.contains("restored_wal_records_checked=1"));

    let payload = run(vec![
        "cortexdb".to_owned(),
        "get".to_owned(),
        restored_arg,
        "43".to_owned(),
    ])
    .unwrap();
    assert_eq!(payload, "backup drill payload");

    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn backup_prune_command_removes_old_matching_backups() {
    let root = unique_path("cortexdb-cli-backup-prune-root");
    std::fs::create_dir_all(&root).unwrap();
    for name in [
        "cortexdb-20260528T000000Z",
        "cortexdb-20260529T000000Z",
        "cortexdb-20260530T000000Z",
    ] {
        let dir = root.join(name);
        std::fs::create_dir(&dir).unwrap();
        std::fs::write(dir.join("marker"), name.as_bytes()).unwrap();
    }

    let output = run(vec![
        "cortexdb".to_owned(),
        "backup-prune".to_owned(),
        root.to_string_lossy().into_owned(),
        "cortexdb-".to_owned(),
        "2".to_owned(),
    ])
    .unwrap();

    assert!(output.contains("backups_seen=3"));
    assert!(output.contains("backups_removed=1"));
    assert!(!root.join("cortexdb-20260528T000000Z").exists());
    assert!(root.join("cortexdb-20260529T000000Z").exists());
    assert!(root.join("cortexdb-20260530T000000Z").exists());

    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn backup_offsite_stage_command_validates_and_publishes_copy() {
    let root = unique_path("cortexdb-cli-backup-offsite-root");
    let source = root.join("source");
    let backup = root.join("backup");
    let offsite = root.join("offsite");
    let source_arg = source.to_string_lossy().into_owned();
    let backup_arg = backup.to_string_lossy().into_owned();
    let offsite_arg = offsite.to_string_lossy().into_owned();

    run(vec![
        "cortexdb".to_owned(),
        "put".to_owned(),
        source_arg.clone(),
        "44".to_owned(),
        "offsite cli payload".to_owned(),
    ])
    .unwrap();
    run(vec![
        "cortexdb".to_owned(),
        "backup".to_owned(),
        source_arg,
        backup_arg.clone(),
    ])
    .unwrap();

    let output = run(vec![
        "cortexdb".to_owned(),
        "backup-offsite-stage".to_owned(),
        backup_arg,
        offsite_arg.clone(),
        "cortexdb-20260530T000000Z".to_owned(),
    ])
    .unwrap();

    assert!(output.contains("target_path="));
    assert!(output.contains("staged_cells_checked="));

    let payload = run(vec![
        "cortexdb".to_owned(),
        "get".to_owned(),
        offsite
            .join("cortexdb-20260530T000000Z")
            .to_string_lossy()
            .into_owned(),
        "44".to_owned(),
    ])
    .unwrap();
    assert_eq!(payload, "offsite cli payload");

    let _ = std::fs::remove_dir_all(root);
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
