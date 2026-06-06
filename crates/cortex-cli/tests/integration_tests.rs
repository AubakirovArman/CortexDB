use std::process::Command;

fn cortexdb_bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_cortexdb"))
}

#[test]
fn cli_version_works() {
    let output = cortexdb_bin().arg("--version").output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("cortexdb"));
    assert!(output.status.success());
}

#[test]
fn cli_help_works() {
    let output = cortexdb_bin().arg("--help").output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("CortexDB local CLI"));
    assert!(stdout.contains("put"));
    assert!(stdout.contains("get"));
    assert!(stdout.contains("flush"));
    assert!(stdout.contains("stats"));
    assert!(stdout.contains("validate"));
    assert!(output.status.success());
}

#[test]
fn cli_put_get_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().to_str().unwrap();

    let put = cortexdb_bin()
        .args(["put", path, "1", "hello"])
        .output()
        .unwrap();
    assert!(
        put.status.success(),
        "{}",
        String::from_utf8_lossy(&put.stderr)
    );

    let get = cortexdb_bin().args(["get", path, "1"]).output().unwrap();
    let stdout = String::from_utf8_lossy(&get.stdout);
    assert!(stdout.contains("hello"));
    assert!(get.status.success());
}

#[test]
fn cli_stats_reports_seq() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().to_str().unwrap();

    let put = cortexdb_bin()
        .args(["put", path, "1", "hello"])
        .output()
        .unwrap();
    assert!(put.status.success());

    let stats = cortexdb_bin().args(["stats", path]).output().unwrap();
    let stdout = String::from_utf8_lossy(&stats.stdout);
    assert!(stdout.contains("current_seq"));
    assert!(stdout.contains("memtable_cells"));
    assert!(stats.status.success());
}

#[test]
fn cli_validate_reports_ok() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().to_str().unwrap();

    let put = cortexdb_bin()
        .args(["put", path, "1", "hello"])
        .output()
        .unwrap();
    assert!(put.status.success());

    let validate = cortexdb_bin().args(["validate", path]).output().unwrap();
    let stdout = String::from_utf8_lossy(&validate.stdout);
    assert!(stdout.contains("ok"));
    assert!(validate.status.success());
}

#[test]
fn cli_restore_dry_run_reports_without_creating_target() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("db");
    let backup = dir.path().join("backup");
    let target = dir.path().join("target");
    let db = db.to_str().unwrap();
    let backup_arg = backup.to_str().unwrap();
    let target_arg = target.to_str().unwrap();

    let put = cortexdb_bin()
        .args(["put", db, "1", "scope=ops\nstatus=ready\npayload"])
        .output()
        .unwrap();
    assert!(put.status.success());

    let backup_output = cortexdb_bin()
        .args(["backup", db, backup_arg])
        .output()
        .unwrap();
    assert!(backup_output.status.success());

    let dry_run = cortexdb_bin()
        .args(["restore", backup_arg, target_arg, "--dry-run"])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&dry_run.stdout);
    assert!(
        dry_run.status.success(),
        "{}",
        String::from_utf8_lossy(&dry_run.stderr)
    );
    assert!(stdout.contains("dry_run=true"));
    assert!(stdout.contains("files_checked="));
    assert!(stdout.contains("backup_wal_records_checked="));
    assert!(!target.exists());
}
