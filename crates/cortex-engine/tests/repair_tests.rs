use std::io::Write;

use cortex_core::CellId;
use cortex_engine::{Database, EngineError};

#[test]
fn repair_best_effort_removes_orphans_and_truncates_wal_tail() {
    let dir = tempfile::tempdir().unwrap();
    {
        let mut db = Database::open(dir.path()).unwrap();
        db.put_cell(CellId(1), b"hello".to_vec()).unwrap();
    }
    std::fs::write(dir.path().join("db.aclog.tmp"), b"orphan").unwrap();
    let wal_path = dir.path().join("db.aclog");
    let before_valid_tail = std::fs::metadata(&wal_path).unwrap().len();
    std::fs::OpenOptions::new()
        .append(true)
        .open(&wal_path)
        .unwrap()
        .write_all(b"partial-tail")
        .unwrap();

    let report = Database::repair_best_effort(dir.path()).unwrap();

    assert_eq!(report.orphan_temp_files_removed, 1);
    assert_eq!(report.wal_records_preserved, 1);
    assert_eq!(report.wal_safe_truncate_offset, before_valid_tail);
    assert!(report.wal_truncated);
    assert!(!dir.path().join("db.aclog.tmp").exists());

    let db = Database::open(dir.path()).unwrap();
    assert_eq!(db.get_latest_cell(CellId(1)), Some(b"hello".to_vec()));
}

#[test]
fn repair_best_effort_respects_active_database_lock() {
    let dir = tempfile::tempdir().unwrap();
    let _db = Database::open(dir.path()).unwrap();

    assert!(matches!(
        Database::repair_best_effort(dir.path()).unwrap_err(),
        EngineError::DatabaseAlreadyOpen(_)
    ));
}
