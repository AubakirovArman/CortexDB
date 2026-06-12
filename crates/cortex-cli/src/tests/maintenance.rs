use super::helpers::*;

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
    assert!(validation.contains("known_sections=3"));

    let dump = run(vec![
        "cortexdb".to_owned(),
        "wal-dump".to_owned(),
        path_arg.clone(),
    ])
    .unwrap();
    assert!(dump.contains("type=PutCellBatch"));
    assert!(dump.contains("sections=3"));

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
