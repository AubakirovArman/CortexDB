use cortex_core::CellId;
use cortex_engine::{Database, EngineError};

#[test]
fn drop_database_shutdowns_writer_and_releases_lock() {
    let dir = tempfile::tempdir().unwrap();
    {
        let mut db = Database::open(dir.path()).unwrap();
        db.put_cell(CellId(1), b"hello".to_vec()).unwrap();
    }
    let db = Database::open(dir.path()).unwrap();
    assert_eq!(db.get_latest_cell(CellId(1)).unwrap(), b"hello");
}

#[test]
fn close_database_shutdowns_writer_and_releases_lock() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_cell(CellId(1), b"hello".to_vec()).unwrap();
    db.close().unwrap();

    let db = Database::open(dir.path()).unwrap();
    assert_eq!(db.get_latest_cell(CellId(1)).unwrap(), b"hello");
}

#[test]
fn second_open_same_path_fails_until_first_database_drops() {
    let dir = tempfile::tempdir().unwrap();
    let db = Database::open(dir.path()).unwrap();
    assert!(matches!(
        Database::open(dir.path()).unwrap_err(),
        EngineError::DatabaseAlreadyOpen(_)
    ));
    drop(db);
    assert!(Database::open(dir.path()).is_ok());
}

#[test]
fn orphan_tmp_files_do_not_break_database_open() {
    let dir = tempfile::tempdir().unwrap();
    let segments = dir.path().join("segments");
    std::fs::create_dir_all(&segments).unwrap();
    std::fs::write(dir.path().join("manifest.acm.tmp"), b"bad").unwrap();
    std::fs::write(dir.path().join("db.aclog.tmp"), b"bad").unwrap();
    std::fs::write(segments.join("segment-1.acs.tmp"), b"bad").unwrap();
    std::fs::write(segments.join("segment-1.acb.tmp"), b"bad").unwrap();
    std::fs::write(segments.join("segment-1.aci.tmp"), b"bad").unwrap();

    let db = Database::open(dir.path()).unwrap();
    assert!(db.validate_storage().is_ok());
    assert!(!dir.path().join("manifest.acm.tmp").exists());
    assert!(!dir.path().join("db.aclog.tmp").exists());
    assert!(!segments.join("segment-1.acs.tmp").exists());
    assert!(!segments.join("segment-1.acb.tmp").exists());
    assert!(!segments.join("segment-1.aci.tmp").exists());
}
