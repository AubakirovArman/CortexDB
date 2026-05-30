use cortex_core::CellId;
use cortex_engine::{Database, EngineError};

#[test]
fn backup_restore_preserves_wal_tail_without_checkpoint() {
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("source");
    let backup = root.path().join("backup");
    let target = root.path().join("target");

    let mut db = Database::open(&source).unwrap();
    db.put_cell(CellId(1), b"wal-only payload".to_vec())
        .unwrap();
    let report = db.backup_to(&backup).unwrap();
    assert!(report.files_copied > 0);
    assert!(report.bytes_copied > 0);

    db.put_cell(CellId(2), b"after backup".to_vec()).unwrap();
    drop(db);

    let restored = Database::restore_from_backup(&backup, &target).unwrap();
    assert!(restored.files_copied > 0);
    assert_eq!(restored.restored_validation.wal_records_checked, 1);

    let db = Database::open(&target).unwrap();
    assert_eq!(db.get_latest_cell(CellId(1)).unwrap(), b"wal-only payload");
    assert_eq!(db.get_latest_cell(CellId(2)), None);
}

#[test]
fn backup_restore_preserves_checkpointed_segment() {
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("source");
    let backup = root.path().join("backup");
    let target = root.path().join("target");

    {
        let mut db = Database::open(&source).unwrap();
        db.put_cell(
            CellId(10),
            b"scope=project:investments\nstatus=ready\ncheckpointed".to_vec(),
        )
        .unwrap();
        db.checkpoint().unwrap();
    }

    let backup_report = Database::backup_path(&source, &backup).unwrap();
    assert_eq!(backup_report.source_validation.live_segments_checked, 1);

    let restore_report = Database::restore_from_backup(&backup, &target).unwrap();
    assert_eq!(restore_report.restored_validation.live_segments_checked, 1);

    let db = Database::open(&target).unwrap();
    assert_eq!(
        db.get_latest_cell(CellId(10)).unwrap(),
        b"scope=project:investments\nstatus=ready\ncheckpointed"
    );
    db.validate_storage().unwrap();
}

#[test]
fn backup_restore_drill_proves_recovered_database_is_readable() {
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("source");
    let backup = root.path().join("backup");
    let drill_target = root.path().join("drill-target");

    {
        let mut db = Database::open(&source).unwrap();
        db.put_cell(CellId(7), b"drill wal tail".to_vec()).unwrap();
        db.put_cell(
            CellId(8),
            b"scope=project:ops\nstatus=ready\ndrill checkpointed".to_vec(),
        )
        .unwrap();
        db.checkpoint().unwrap();
        db.put_cell(CellId(9), b"drill post-checkpoint tail".to_vec())
            .unwrap();
    }

    let report = Database::backup_restore_drill_path(&source, &backup, &drill_target).unwrap();
    assert!(report.backup.files_copied > 0);
    assert!(report.restore.files_copied > 0);
    assert_eq!(report.restore.restored_validation.live_segments_checked, 1);
    assert_eq!(report.restore.restored_validation.wal_records_checked, 1);

    let restored = Database::open(&drill_target).unwrap();
    assert_eq!(
        restored.get_latest_cell(CellId(7)).unwrap(),
        b"drill wal tail"
    );
    assert_eq!(
        restored.get_latest_cell(CellId(8)).unwrap(),
        b"scope=project:ops\nstatus=ready\ndrill checkpointed"
    );
    assert_eq!(
        restored.get_latest_cell(CellId(9)).unwrap(),
        b"drill post-checkpoint tail"
    );
}

#[test]
fn restore_rejects_existing_target() {
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("source");
    let backup = root.path().join("backup");
    let target = root.path().join("target");

    {
        let mut db = Database::open(&source).unwrap();
        db.put_cell(CellId(1), b"payload".to_vec()).unwrap();
    }
    Database::backup_path(&source, &backup).unwrap();
    std::fs::create_dir(&target).unwrap();

    assert!(matches!(
        Database::restore_from_backup(&backup, &target).unwrap_err(),
        EngineError::BackupTargetExists(_)
    ));
}

#[test]
fn backup_rejects_target_inside_source() {
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("source");
    let backup = source.join("nested-backup");

    let mut db = Database::open(&source).unwrap();
    db.put_cell(CellId(1), b"payload".to_vec()).unwrap();

    let error = db.backup_to(&backup).unwrap_err().to_string();
    assert!(error.contains("backup target must not be inside source database"));
}

#[test]
fn backup_retention_prunes_oldest_matching_backups() {
    let root = tempfile::tempdir().unwrap();
    for name in [
        "cortexdb-20260528T000000Z",
        "cortexdb-20260529T000000Z",
        "cortexdb-20260530T000000Z",
        "other-20260527T000000Z",
    ] {
        let dir = root.path().join(name);
        std::fs::create_dir(&dir).unwrap();
        std::fs::write(dir.join("marker"), name.as_bytes()).unwrap();
    }

    let report = Database::prune_backup_retention(root.path(), "cortexdb-", 2).unwrap();

    assert_eq!(report.backups_seen, 3);
    assert_eq!(report.backups_kept, 2);
    assert_eq!(report.backups_removed, 1);
    assert!(report.bytes_removed > 0);
    assert!(!root.path().join("cortexdb-20260528T000000Z").exists());
    assert!(root.path().join("cortexdb-20260529T000000Z").exists());
    assert!(root.path().join("cortexdb-20260530T000000Z").exists());
    assert!(root.path().join("other-20260527T000000Z").exists());
}

#[test]
fn backup_retention_rejects_unsafe_plan() {
    let root = tempfile::tempdir().unwrap();

    let empty_prefix = Database::prune_backup_retention(root.path(), "", 1)
        .unwrap_err()
        .to_string();
    assert!(empty_prefix.contains("prefix must not be empty"));

    let zero_keep = Database::prune_backup_retention(root.path(), "cortexdb-", 0)
        .unwrap_err()
        .to_string();
    assert!(zero_keep.contains("keep_latest must be greater than zero"));
}
