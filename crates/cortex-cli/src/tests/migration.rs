use super::helpers::*;

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
fn migrate_offline_creates_backup_drill_rewrites_and_preserves_data() {
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
    assert!(output.contains(r#""phase":"migrate_offline""#));
    assert!(output.contains(r#""status":"offline_migration_completed""#));
    assert!(output.contains(r#""migration_segment_id":1"#));
    assert!(output.contains(r#""migration_cells_rewritten":1"#));
    assert!(output.contains(r#""post_migration_cells_checked":1"#));
    assert!(output.contains(r#""validate_after_migration_command""#));
    assert!(output.contains(r#""rollback_command""#));

    let segment_magic = std::fs::read(source.join("segments").join("segment-1.acs"))
        .unwrap()
        .into_iter()
        .take(4)
        .collect::<Vec<_>>();
    assert_eq!(segment_magic.as_slice(), b"ACS2");

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
