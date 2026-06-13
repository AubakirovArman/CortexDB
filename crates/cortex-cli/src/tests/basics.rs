use super::helpers::*;

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
    assert!(help.contains("init"));
    assert!(help.contains("ingest-json"));
    assert!(help.contains("doctor"));
    assert!(help.contains("completions"));

    let version = run(vec!["cortexdb".to_owned(), "version".to_owned()]).unwrap();
    assert!(version.starts_with("cortexdb "));
}

#[test]
fn init_creates_quickstart_database_agent_view_and_sample_context() {
    let path = unique_path("cortexdb-cli-init");
    let path_arg = path.to_string_lossy().into_owned();

    let init = run(vec![
        "cortexdb".to_owned(),
        "init".to_owned(),
        path_arg.clone(),
    ])
    .unwrap();
    assert!(init.contains("CortexDB initialized"));
    assert!(init.contains("agent_view_id=1"));
    assert!(init.contains("sample_scope=project:starter"));
    assert!(init.contains("cortexdb doctor"));
    assert!(path.join("agent_views/1.view").exists());

    let doctor = run(vec![
        "cortexdb".to_owned(),
        "doctor".to_owned(),
        path_arg.clone(),
    ])
    .unwrap();
    assert!(doctor.contains("wal:"));
    assert!(doctor.contains("format_versions:"));
    assert!(doctor.contains("memory_forecast:"));
    assert!(doctor.contains("All checks passed"));

    let context = run(vec![
        "cortexdb".to_owned(),
        "context".to_owned(),
        path_arg.clone(),
        "project:starter".to_owned(),
        "RETRIEVE CONTEXT FOR TASK \"starter onboarding\" IN BRAIN default LIMIT 5 CANDIDATES;"
            .to_owned(),
    ])
    .unwrap();
    assert!(context.contains("cells=1"));
    assert!(context.contains("ContextPack retrieval"));

    let _ = std::fs::remove_dir_all(path);
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
    assert!(doctor.contains("wal:"));
    assert!(doctor.contains("format_versions:"));
    assert!(doctor.contains("memory_forecast:"));
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
    assert!(bash.contains("init"));
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
    assert!(doctor.contains("wal:"));
    assert!(doctor.contains("format_versions:"));
    assert!(doctor.contains("memory_forecast:"));
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
        "init",
        "doctor",
        "stats",
        "validate",
        "vector",
        "context",
        "agent",
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
