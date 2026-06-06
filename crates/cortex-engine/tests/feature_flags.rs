use std::collections::BTreeSet;

use cortex_aql::{AgentId, AgentView, BrainId, MemoryType, RetrievalMode, Q16_ZERO};
use cortex_core::CellId;
use cortex_engine::{
    scope_id, Database, DatabaseOptions, EngineError, EngineFeature, EngineFeatureFlags,
    SearchLimit,
};
use cortex_engine::{AnnFallbackReason, AnnSearchPath};

fn view(scope: &str) -> AgentView {
    let scope_id = scope_id(scope);
    AgentView {
        agent_id: AgentId(1),
        label: Some("feature-flag-test".to_owned()),
        readable_brains: BTreeSet::from([BrainId(1)]),
        readable_scopes: BTreeSet::from([scope_id]),
        writable_scopes: BTreeSet::from([scope_id]),
        allowed_modes: BTreeSet::from([RetrievalMode::Balanced]),
        allowed_memory_types: BTreeSet::from([MemoryType::Decision]),
        max_context_budget_tokens: 4_000,
        default_context_budget_tokens: 1_000,
        max_candidate_limit: 32,
        allow_remember: true,
        allow_verify_fact: true,
        allow_audit_mode: true,
        private_scope: Some(scope_id),
        max_ttl_seconds: None,
        default_candidate_limit: 20,
        min_required_confidence_q16: Q16_ZERO,
        require_citations_by_default: false,
    }
}

fn hnsw_options() -> DatabaseOptions {
    DatabaseOptions {
        feature_flags: EngineFeatureFlags::production_safe().with_experimental_hnsw(true),
        ..DatabaseOptions::default()
    }
}

#[test]
fn engine_feature_flags_default_to_production_safe() {
    let flags = DatabaseOptions::default().feature_flags;
    assert!(!flags.is_enabled(EngineFeature::ExperimentalHnsw));
    assert!(!flags.is_enabled(EngineFeature::ExperimentalReplication));
    assert!(!flags.is_enabled(EngineFeature::Dashboard));
    assert_eq!(
        EngineFeature::ExperimentalHnsw.as_str(),
        "experimental_hnsw"
    );
}

#[test]
fn default_checkpoint_skips_hnsw_and_vector_search_uses_exact_fallback() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_cell(
        CellId(1),
        b"scope=project:investments\nstatus=ready\nvector=10,0\n\nalpha".to_vec(),
    )
    .unwrap();
    db.put_cell(
        CellId(2),
        b"scope=project:investments\nstatus=ready\nvector=0,10\n\nbeta".to_vec(),
    )
    .unwrap();
    db.checkpoint().unwrap();

    assert!(db.manifest().hnsw_profile.is_none());
    assert!(!dir.path().join("segments/segment-1.ach").exists());
    assert!(db.validate_storage_report().errors.is_empty());

    let outcome = db
        .search_vector_with_report(&[10, 0], &view("project:investments"), SearchLimit(2))
        .unwrap();
    assert_eq!(outcome.results[0].cell_id, CellId(1));
    let report = outcome.ann_report.expect("vector report");
    assert_eq!(report.path, AnnSearchPath::ExactFallback);
    assert_eq!(
        report.fallback_reason,
        Some(AnnFallbackReason::HnswDisabled)
    );
    assert!(report.production_safe);
}

#[test]
fn experimental_hnsw_flag_persists_graph_profile() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open_with_options(dir.path(), hnsw_options()).unwrap();
    db.put_cell(
        CellId(1),
        b"scope=project:investments\nstatus=ready\nvector=10,0\n\nalpha".to_vec(),
    )
    .unwrap();
    db.checkpoint().unwrap();

    assert!(db.manifest().hnsw_profile.is_some());
    assert!(dir.path().join("segments/segment-1.ach").exists());
}

#[test]
fn replication_database_surface_requires_feature_flag() {
    let dir = tempfile::tempdir().unwrap();
    let db = Database::open(dir.path()).unwrap();
    let error = db.replication_snapshot_segment().unwrap_err();
    assert!(matches!(
        error,
        EngineError::FeatureDisabled("experimental_replication")
    ));
}
