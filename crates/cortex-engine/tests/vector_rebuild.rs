use std::collections::{BTreeMap, BTreeSet};

use cortex_aql::{AgentId, AgentView, BrainId, MemoryType, RetrievalMode, Q16_ZERO};
use cortex_core::CellId;
use cortex_engine::{
    scope_id, AnnFallbackReason, AnnSearchPath, Database, DatabaseOptions, EngineFeatureFlags,
    SearchLimit,
};
use cortex_storage::hnsw::HnswGraphIndex;

#[test]
fn vector_rebuild_repairs_corrupt_vector_index() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = hnsw_database(dir.path());
    put_two_vectors(&mut db);
    db.checkpoint().unwrap();

    let vector_path = dir.path().join("segments/segment-1.acv");
    let mut bytes = std::fs::read(&vector_path).unwrap();
    *bytes.last_mut().unwrap() ^= 0xff;
    std::fs::write(&vector_path, bytes).unwrap();
    assert!(db.validate_storage().is_err());

    let report = db.rebuild_vector_indexes(true).unwrap();

    assert_eq!(report.segments_checked, 1);
    assert_eq!(report.vector_indexes_rebuilt, 1);
    assert_eq!(report.hnsw_graphs_rebuilt, 1);
    assert_eq!(report.vector_candidates, 2);
    db.validate_storage().unwrap();
    assert_hnsw_search_is_healthy(&db);
}

#[test]
fn vector_rebuild_repairs_corrupt_hnsw_graph() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = hnsw_database(dir.path());
    put_two_vectors(&mut db);
    db.checkpoint().unwrap();

    let graph_path = dir.path().join("segments/segment-1.ach");
    let mut bytes = std::fs::read(&graph_path).unwrap();
    bytes.truncate(bytes.len().saturating_sub(4));
    std::fs::write(&graph_path, bytes).unwrap();
    assert!(db.validate_storage().is_err());

    let report = db.rebuild_vector_indexes(true).unwrap();

    assert_eq!(report.hnsw_graphs_rebuilt, 1);
    db.validate_storage().unwrap();
    assert_hnsw_search_is_healthy(&db);
}

#[test]
fn vector_rebuild_repairs_stale_hnsw_graph() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = hnsw_database(dir.path());
    put_two_vectors(&mut db);
    db.checkpoint().unwrap();

    HnswGraphIndex {
        links: BTreeMap::from([(1, BTreeSet::new())]),
        dimension: 2,
        metric: 0,
        max_neighbors: 16,
        ef_search: 128,
        layer_count: 4,
        ef_construction: 128,
        ..HnswGraphIndex::default()
    }
    .write(dir.path().join("segments/segment-1.ach"))
    .unwrap();

    let stale = db
        .search_vector_with_report(&[0, 10], &view("project:investments"), SearchLimit(1))
        .unwrap()
        .ann_report
        .unwrap();
    assert_eq!(stale.fallback_reason, Some(AnnFallbackReason::StaleGraph));

    let report = db.rebuild_vector_indexes(true).unwrap();

    assert_eq!(report.hnsw_graphs_rebuilt, 1);
    db.validate_storage().unwrap();
    assert_hnsw_search_is_healthy(&db);
}

fn assert_hnsw_search_is_healthy(db: &Database) {
    let outcome = db
        .search_vector_with_report(&[0, 10], &view("project:investments"), SearchLimit(1))
        .unwrap();
    assert_eq!(outcome.results[0].cell_id, CellId(2));
    let report = outcome.ann_report.expect("ann report");
    assert_eq!(report.path, AnnSearchPath::HnswGraph);
    assert_eq!(report.fallback_reason, None);
    assert!(!report.fallback_performed);
}

fn hnsw_database(path: &std::path::Path) -> Database {
    Database::open_with_options(
        path,
        DatabaseOptions {
            feature_flags: EngineFeatureFlags::production_safe().with_experimental_hnsw(true),
            ..DatabaseOptions::default()
        },
    )
    .unwrap()
}

fn put_two_vectors(db: &mut Database) {
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
}

fn view(scope: &str) -> AgentView {
    let scope_id = scope_id(scope);
    AgentView {
        agent_id: AgentId(1),
        label: Some("vector-rebuild".to_owned()),
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
