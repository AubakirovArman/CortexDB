use super::run;

#[test]
fn wal_truncate_command_reports_safe_offset() {
    let path = unique_path("cortexdb-cli-wal-truncate");
    let path_arg = path.to_string_lossy().into_owned();
    run(vec![
        "cortexdb".to_owned(),
        "put".to_owned(),
        path_arg.clone(),
        "1".to_owned(),
        "hello".to_owned(),
    ])
    .unwrap();

    let output = run(vec![
        "cortexdb".to_owned(),
        "wal-truncate".to_owned(),
        path_arg.clone(),
    ])
    .unwrap();
    assert!(output.contains("wal_records_preserved=1"));
    assert!(output.contains("wal_truncated=false"));

    let _ = std::fs::remove_dir_all(path);
}

#[test]
fn manifest_dump_and_validate_commands_report_segments() {
    let path = unique_path("cortexdb-cli-manifest");
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

    let validation = run(vec![
        "cortexdb".to_owned(),
        "manifest-validate".to_owned(),
        path_arg.clone(),
    ])
    .unwrap();
    assert!(validation.contains("generation=1"));
    assert!(validation.contains("live_segments=1"));

    let dump = run(vec![
        "cortexdb".to_owned(),
        "manifest-dump".to_owned(),
        path_arg.clone(),
    ])
    .unwrap();
    assert!(dump.contains("checkpoint_seq=1"));
    assert!(dump.contains("live id=1"));

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
