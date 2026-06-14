use cortex_core::CellId;
use cortex_engine::{
    AqlQueryCacheStats, BackupReport, CandidateId, CheckpointStats, ContextPack,
    ContextPackOptions, Database, DatabaseOptions, DbOperation, EngineAqlIndex, EngineConfig,
    EngineConfigError, EngineError, EngineErrorCategory, EngineErrorCode, EngineFeature,
    EngineFeatureFlags, EngineResult, Language, RecoveryMode, RepairReport, RestoreReport,
    RetrievedCell, StaleLockPolicy, StorageStats, StorageValidationReport, TextAnalyzer,
    TextAnalyzerConfig,
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
    let _candidate = CandidateId(1);
    let _index = EngineAqlIndex::default();
    let _ = std::mem::size_of::<BackupReport>();
    let _ = std::mem::size_of::<CheckpointStats>();
    let _ = std::mem::size_of::<ContextPack>();
    let _ = std::mem::size_of::<EngineError>();
    let _ = EngineErrorCode::BadRequest.as_str();
    let _ = EngineErrorCategory::UserInput.as_str();
    let _ = EngineFeature::ExperimentalHnsw.as_str();
    let _ = EngineFeatureFlags::production_safe();
    let analyzer_config = TextAnalyzerConfig {
        language: Language::Russian,
        stemming: true,
    };
    let analyzer = TextAnalyzer::with_config(analyzer_config);
    assert!(analyzer.tokenize("бюджету").contains(&"бюджет".to_owned()));
    let config = EngineConfig::from_env_vars([("CORTEXDB_DURABILITY_MODE", "strict")]).unwrap();
    assert_eq!(config.database_options.recovery_mode, RecoveryMode::Strict);
    let _ = std::mem::size_of::<EngineConfigError>();
    let _ = std::mem::size_of::<RestoreReport>();
    let _ = std::mem::size_of::<RetrievedCell>();
    let _ = std::mem::size_of::<StorageStats>();
    let _ = std::mem::size_of::<AqlQueryCacheStats>();
}
