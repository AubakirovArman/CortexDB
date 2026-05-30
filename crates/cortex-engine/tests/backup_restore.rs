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
