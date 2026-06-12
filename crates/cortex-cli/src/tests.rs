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
    assert!(help.contains("doctor"));
    assert!(help.contains("completions"));

    let version = run(vec!["cortexdb".to_owned(), "version".to_owned()]).unwrap();
    assert!(version.starts_with("cortexdb "));
}

#[test]
fn doctor_and_completions_commands_work() {
    let path = unique_path("cortexdb-cli-doctor");
    let path_arg = path.to_string_lossy().into_owned();
    run(vec![
        "cortexdb".to_owned(),
        "put".to_owned(),
        path_arg.clone(),
        "1".to_owned(),
        "scope=project:investments\nstatus=ready\nhealth payload".to_owned(),
    ])
    .unwrap();

    let doctor = run(vec![
        "cortexdb".to_owned(),
        "doctor".to_owned(),
        path_arg.clone(),
    ])
    .unwrap();
    assert!(doctor.contains("CortexDB Doctor Report"));
    assert!(doctor.contains("open: database opened successfully"));
    assert!(doctor.contains("tenant: tenant=default"));
    assert!(doctor.contains("db_lock: lock acquired"));
    assert!(doctor.contains("backup_age:"));
    assert!(doctor.contains("server_health:"));
    assert!(doctor.contains("auth:"));
    assert!(doctor.contains("repair_advice:"));
    assert!(doctor.contains("All checks passed"));

    let bash = run(vec![
        "cortexdb".to_owned(),
        "completions".to_owned(),
        "bash".to_owned(),
    ])
    .unwrap();
    assert!(bash.contains("_cortexdb"));
    assert!(bash.contains("doctor"));
    assert!(bash.contains("completions"));

    let _ = std::fs::remove_dir_all(path);
}

#[test]
fn doctor_reports_lock_backup_server_auth_tenant_and_repair_advice() {
    let path = unique_path("cortexdb-cli-doctor-expanded");
    let path_arg = path.to_string_lossy().into_owned();
    let tenant = "tenant_alpha";
    let doctor = run(vec![
        "cortexdb".to_owned(),
        "--tenant".to_owned(),
        tenant.to_owned(),
        "doctor".to_owned(),
        path_arg.clone(),
    ])
    .unwrap();

    assert!(doctor.contains("tenant: tenant=tenant_alpha"));
    assert!(doctor.contains("db_lock: lock acquired"));
    assert!(doctor.contains("validate:"));
    assert!(doctor.contains("backup_age:"));
    assert!(doctor.contains("server_health:"));
    assert!(doctor.contains("auth:"));
    assert!(doctor.contains("repair_advice: no repair needed"));
    assert!(doctor.contains("All checks passed"));

    let error = run(vec![
        "cortexdb".to_owned(),
        "--tenant".to_owned(),
        "../escape".to_owned(),
        "doctor".to_owned(),
        path_arg,
    ])
    .unwrap_err();
    assert!(error.contains("tenant is invalid"));

    let _ = std::fs::remove_dir_all(path);
}

#[test]
fn doctor_reports_stale_lock_repair_advice() {
    let path = unique_path("cortexdb-cli-doctor-stale-lock");
    std::fs::create_dir_all(&path).unwrap();
    std::fs::write(path.join("db.lock"), b"stale").unwrap();
    let path_arg = path.to_string_lossy().into_owned();

    let doctor = run(vec![
        "cortexdb".to_owned(),
        "doctor".to_owned(),
        path_arg.clone(),
    ])
    .unwrap();
    assert!(doctor.contains("open: failed to open"));
    assert!(doctor.contains("db_lock: lock exists"));
    assert!(doctor.contains("cortexdb unlock"));
    assert!(doctor.contains("repair_advice: run cortexdb repair --dry-run"));
    assert!(doctor.contains("Some checks failed"));

    let _ = std::fs::remove_dir_all(path);
}

#[test]
fn cli_golden_outputs_are_stable() {
    let help = run(vec!["cortexdb".to_owned(), "--help".to_owned()]).unwrap();
    for marker in [
        "Usage: cortexdb",
        "Commands:",
        "doctor",
        "stats",
        "validate",
        "vector",
        "context",
        "verify",
        "search-vector-eval",
        "migrate",
        "Command groups:",
        "Core database:",
        "Agent retrieval:",
        "Vector and ANN:",
    ] {
        assert!(help.contains(marker), "missing help marker: {marker}");
    }

    let context_help = run(vec![
        "cortexdb".to_owned(),
        "help".to_owned(),
        "context".to_owned(),
    ])
    .unwrap();
    assert!(context_help.contains("Build a token-budgeted"));
    assert!(context_help.contains("RETRIEVE CONTEXT FOR TASK"));

    let version = run(vec!["cortexdb".to_owned(), "version".to_owned()]).unwrap();
    assert!(version.starts_with("cortexdb "));
}

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
fn repair_dry_run_reports_without_mutating() {
    let path = unique_path("cortexdb-cli-repair-dry-run");
    let path_arg = path.to_string_lossy().into_owned();
    std::fs::create_dir_all(&path).unwrap();
    std::fs::write(path.join("db.aclog.tmp"), b"bad").unwrap();

    let output = run(vec![
        "cortexdb".to_owned(),
        "repair".to_owned(),
        "--dry-run".to_owned(),
        path_arg.clone(),
    ])
    .unwrap();
    assert!(output.contains("dry_run=true"));
    assert!(output.contains("orphan_temp_files_removed=1"));
    assert!(path.join("db.aclog.tmp").exists());

    let apply = run(vec![
        "cortexdb".to_owned(),
        "repair".to_owned(),
        path_arg.clone(),
    ])
    .unwrap();
    assert!(apply.contains("dry_run=false"));
    assert!(apply.contains("orphan_temp_files_removed=1"));
    assert!(!path.join("db.aclog.tmp").exists());

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
fn backup_encrypted_and_restore_encrypted_commands_roundtrip_database() {
    let root = unique_path("cortexdb-cli-backup-encrypted-root");
    let source = root.join("source");
    let archive = root.join("backup.cdbenc");
    let target = root.join("target");
    let source_arg = source.to_string_lossy().into_owned();
    let archive_arg = archive.to_string_lossy().into_owned();
    let target_arg = target.to_string_lossy().into_owned();
    let passphrase = "cli encrypted backup passphrase";

    run(vec![
        "cortexdb".to_owned(),
        "put".to_owned(),
        source_arg.clone(),
        "142".to_owned(),
        "encrypted backup payload".to_owned(),
    ])
    .unwrap();

    let backup_output = run(vec![
        "cortexdb".to_owned(),
        "backup-encrypted".to_owned(),
        source_arg,
        archive_arg.clone(),
        "--passphrase".to_owned(),
        passphrase.to_owned(),
    ])
    .unwrap();
    assert!(backup_output.contains("files_archived="));
    assert!(backup_output.contains("ciphertext_bytes="));

    let restore_output = run(vec![
        "cortexdb".to_owned(),
        "restore-encrypted".to_owned(),
        archive_arg,
        target_arg.clone(),
        "--passphrase".to_owned(),
        passphrase.to_owned(),
    ])
    .unwrap();
    assert!(restore_output.contains("files_restored="));
    assert!(restore_output.contains("restored_wal_records_checked=1"));

    let payload = run(vec![
        "cortexdb".to_owned(),
        "get".to_owned(),
        target_arg,
        "142".to_owned(),
    ])
    .unwrap();
    assert_eq!(payload, "encrypted backup payload");

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
fn upgrade_prepare_validate_and_rollback_flow() {
    let root = unique_path("cortexdb-cli-upgrade-flow-root");
    let source = root.join("source");
    let backup = root.join("pre-upgrade-backup");
    let drill = root.join("pre-upgrade-drill");
    let rollback = root.join("rollback");
    let source_arg = source.to_string_lossy().into_owned();
    let backup_arg = backup.to_string_lossy().into_owned();
    let drill_arg = drill.to_string_lossy().into_owned();
    let rollback_arg = rollback.to_string_lossy().into_owned();

    run(vec![
        "cortexdb".to_owned(),
        "put".to_owned(),
        source_arg.clone(),
        "44".to_owned(),
        "upgrade flow payload".to_owned(),
    ])
    .unwrap();

    let prepare = run(vec![
        "cortexdb".to_owned(),
        "upgrade".to_owned(),
        "prepare".to_owned(),
        source_arg.clone(),
        backup_arg.clone(),
        drill_arg.clone(),
    ])
    .unwrap();
    assert!(prepare.contains("phase=upgrade_prepare"));
    assert!(prepare.contains("status=ready_for_offline_upgrade"));
    assert!(prepare.contains("backup_files_copied="));
    assert!(backup.exists());
    assert!(drill.exists());

    let validate = run(vec![
        "cortexdb".to_owned(),
        "upgrade".to_owned(),
        "validate".to_owned(),
        source_arg,
    ])
    .unwrap();
    assert!(validate.contains("phase=upgrade_validate"));
    assert!(validate.contains("status=validated_after_upgrade"));

    let rollback_output = run(vec![
        "cortexdb".to_owned(),
        "upgrade".to_owned(),
        "rollback".to_owned(),
        backup_arg,
        rollback_arg.clone(),
    ])
    .unwrap();
    assert!(rollback_output.contains("phase=upgrade_rollback"));
    assert!(rollback_output.contains("status=rollback_restored_and_validated"));
    assert!(rollback_output.contains("restored_wal_records_checked=1"));

    let payload = run(vec![
        "cortexdb".to_owned(),
        "get".to_owned(),
        rollback_arg,
        "44".to_owned(),
    ])
    .unwrap();
    assert_eq!(payload, "upgrade flow payload");

    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn upgrade_prepare_json_reports_next_commands() {
    let root = unique_path("cortexdb-cli-upgrade-json-root");
    let source = root.join("source");
    let backup = root.join("backup");
    let drill = root.join("drill");
    let source_arg = source.to_string_lossy().into_owned();
    let backup_arg = backup.to_string_lossy().into_owned();
    let drill_arg = drill.to_string_lossy().into_owned();

    run(vec![
        "cortexdb".to_owned(),
        "put".to_owned(),
        source_arg.clone(),
        "45".to_owned(),
        "upgrade json payload".to_owned(),
    ])
    .unwrap();

    let output = run(vec![
        "cortexdb".to_owned(),
        "--json".to_owned(),
        "upgrade".to_owned(),
        "prepare".to_owned(),
        source_arg,
        backup_arg,
        drill_arg,
    ])
    .unwrap();
    assert!(output.contains(r#""phase":"upgrade_prepare""#));
    assert!(output.contains(r#""status":"ready_for_offline_upgrade""#));
    assert!(output.contains(r#""validate_after_upgrade_command""#));
    assert!(output.contains(r#""rollback_command""#));

    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn migrate_preflight_creates_backup_drill_and_preserves_data() {
    let root = unique_path("cortexdb-cli-migrate-root");
    let source = root.join("source");
    let backup = root.join("migration-backup");
    let drill = root.join("migration-drill");
    let source_arg = source.to_string_lossy().into_owned();
    let backup_arg = backup.to_string_lossy().into_owned();
    let drill_arg = drill.to_string_lossy().into_owned();

    run(vec![
        "cortexdb".to_owned(),
        "put".to_owned(),
        source_arg.clone(),
        "46".to_owned(),
        "migration payload".to_owned(),
    ])
    .unwrap();

    let output = run(vec![
        "cortexdb".to_owned(),
        "--json".to_owned(),
        "migrate".to_owned(),
        source_arg.clone(),
        backup_arg,
        drill_arg.clone(),
    ])
    .unwrap();
    assert!(output.contains(r#""phase":"migrate_preflight""#));
    assert!(output.contains(r#""status":"ready_for_offline_migration""#));
    assert!(output.contains(r#""validate_after_migration_command""#));
    assert!(output.contains(r#""rollback_command""#));

    let source_payload = run(vec![
        "cortexdb".to_owned(),
        "get".to_owned(),
        source_arg,
        "46".to_owned(),
    ])
    .unwrap();
    assert_eq!(source_payload, "migration payload");

    let drill_payload = run(vec![
        "cortexdb".to_owned(),
        "get".to_owned(),
        drill_arg,
        "46".to_owned(),
    ])
    .unwrap();
    assert_eq!(drill_payload, "migration payload");

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
fn backup_prune_dry_run_reports_without_removing() {
    let root = unique_path("cortexdb-cli-backup-prune-dry-run-root");
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
        "--dry-run".to_owned(),
    ])
    .unwrap();

    assert!(output.contains("dry_run=true"));
    assert!(output.contains("backups_seen=3"));
    assert!(output.contains("backups_removed=1"));
    assert!(root.join("cortexdb-20260528T000000Z").exists());
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
    assert!(output.contains("adapter=local_filesystem"));
    assert!(output.contains("published=true"));
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
