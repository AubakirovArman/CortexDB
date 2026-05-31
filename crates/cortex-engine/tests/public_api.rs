use cortex_core::CellId;
use cortex_engine::{
    ContextPackOptions, Database, DatabaseOptions, DbOperation, EngineResult, RecoveryMode,
    RepairReport, StaleLockPolicy, StorageValidationReport,
};

#[test]
fn stable_database_facade_compiles_and_roundtrips() -> EngineResult<()> {
    let dir = tempfile::tempdir().unwrap();
    let options = DatabaseOptions {
        recovery_mode: RecoveryMode::Strict,
        stale_lock_policy: StaleLockPolicy::Reject,
        ..DatabaseOptions::default()
    };
    let mut db = Database::open_with_options(dir.path(), options)?;
    let seq = db.put_cell(CellId(1), b"scope=api\nstatus=ready\nhello".to_vec())?;
    assert_eq!(seq.0, 1);
    assert_eq!(
        db.get_latest_cell(CellId(1)),
        Some(b"scope=api\nstatus=ready\nhello".to_vec())
    );
    db.close()?;

    let db = Database::open(dir.path())?;
    assert_eq!(
        db.get_latest_cell(CellId(1)),
        Some(b"scope=api\nstatus=ready\nhello".to_vec())
    );
    Ok(())
}

#[test]
fn stable_public_types_are_importable() {
    let _operation = DbOperation::PutCell {
        cell_id: CellId(7),
        payload: b"payload".to_vec(),
    };
    let _context_options = ContextPackOptions::default();
    let _repair_report = RepairReport::default();
    let validation_report = StorageValidationReport::default();
    assert!(validation_report.errors.is_empty());
}
