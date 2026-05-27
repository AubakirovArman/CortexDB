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
