use super::helpers::*;

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
fn backup_verify_command_validates_backup_and_catches_corruption() {
    let root = unique_path("cortexdb-cli-backup-verify-root");
    let source = root.join("source");
    let backup = root.join("backup");
    let source_arg = source.to_string_lossy().into_owned();
    let backup_arg = backup.to_string_lossy().into_owned();

    run(vec![
        "cortexdb".to_owned(),
        "put".to_owned(),
        source_arg.clone(),
        "45".to_owned(),
        "backup verify payload".to_owned(),
    ])
    .unwrap();
    run(vec![
        "cortexdb".to_owned(),
        "flush".to_owned(),
        source_arg.clone(),
    ])
    .unwrap();
    run(vec![
        "cortexdb".to_owned(),
        "backup".to_owned(),
        source_arg,
        backup_arg.clone(),
    ])
    .unwrap();
    assert!(backup.join("backup_manifest.tsv").exists());

    let verify = run(vec![
        "cortexdb".to_owned(),
        "backup-verify".to_owned(),
        backup_arg.clone(),
    ])
    .unwrap();
    assert!(verify.contains("backup_ok=true"));
    assert!(verify.contains("checksum_manifest_present=true"));
    assert!(verify.contains("checksum_manifest_files_verified="));
    assert!(verify.contains("backup_live_segments_checked=1"));

    corrupt_last_byte(&backup.join("segments").join("segment-1.acs"));
    let error = run(vec![
        "cortexdb".to_owned(),
        "backup-verify".to_owned(),
        backup_arg,
    ])
    .unwrap_err();
    assert!(
        error.contains("backup manifest checksum mismatch"),
        "{error}"
    );

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

fn corrupt_last_byte(path: &std::path::Path) {
    let mut bytes = std::fs::read(path).unwrap();
    *bytes.last_mut().unwrap() ^= 0xff;
    std::fs::write(path, bytes).unwrap();
}
