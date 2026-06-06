use std::collections::BTreeSet;

use cortex_aql::{AgentId, AgentView, BrainId, MemoryType, RetrievalMode, ScopeId, Q16_ZERO};
use cortex_core::CellId;
use cortex_engine::{scope_id, Database};

#[test]
fn repeated_retrieve_aql_uses_query_cache() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_cell(
        CellId(1),
        b"scope=project:investments\nstatus=ready\nalpha budget".to_vec(),
    )
    .unwrap();
    let readable = view(scope_id("project:investments"));
    let query = r#"RETRIEVE CONTEXT FOR TASK "budget" IN BRAIN investment_projects
WHERE space = project:investments AND status = "ready" LIMIT 10 CANDIDATES;"#;

    assert_eq!(db.aql_query_cache_stats().unwrap().hits, 0);
    db.retrieve_aql(query, &readable).unwrap();
    let after_first = db.aql_query_cache_stats().unwrap();
    assert_eq!(after_first.misses, 1);
    assert_eq!(after_first.hits, 0);
    assert_eq!(after_first.entries, 1);

    db.retrieve_aql(query, &readable).unwrap();
    let after_second = db.aql_query_cache_stats().unwrap();
    assert_eq!(after_second.misses, 1);
    assert_eq!(after_second.hits, 1);
    assert_eq!(after_second.entries, 1);
}

#[test]
fn aql_query_cache_is_scoped_by_agent_view() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_cell(
        CellId(1),
        b"scope=project:a\nstatus=ready\nalpha budget".to_vec(),
    )
    .unwrap();
    let query = r#"RETRIEVE CONTEXT FOR TASK "budget" IN BRAIN investment_projects
WHERE space = project:a AND status = "ready" LIMIT 10 CANDIDATES;"#;

    db.retrieve_aql(query, &view(scope_id("project:a")))
        .unwrap();
    let denied = db.retrieve_aql(query, &view(scope_id("project:b")));
    assert!(denied.is_err());

    let stats = db.aql_query_cache_stats().unwrap();
    assert_eq!(stats.misses, 2);
    assert_eq!(stats.hits, 0);

    db.retrieve_aql(query, &view(scope_id("project:a")))
        .unwrap();
    let stats = db.aql_query_cache_stats().unwrap();
    assert_eq!(stats.misses, 2);
    assert_eq!(stats.hits, 1);
}

#[test]
fn aql_query_cache_invalidates_after_catalog_version_change() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_cell(
        CellId(1),
        b"scope=project:investments\nstatus=ready\nalpha budget".to_vec(),
    )
    .unwrap();
    let readable = view(scope_id("project:investments"));
    let query = r#"RETRIEVE CONTEXT FOR TASK "budget" IN BRAIN investment_projects
WHERE space = project:investments AND status = "ready" LIMIT 10 CANDIDATES;"#;

    db.retrieve_aql(query, &readable).unwrap();
    db.retrieve_aql(query, &readable).unwrap();
    assert_eq!(db.aql_query_cache_stats().unwrap().hits, 1);

    db.put_cell(
        CellId(2),
        b"scope=project:investments\nstatus=ready\nbeta budget".to_vec(),
    )
    .unwrap();
    let cells = db.retrieve_aql(query, &readable).unwrap();
    let stats = db.aql_query_cache_stats().unwrap();
    assert_eq!(cells.len(), 2);
    assert_eq!(stats.hits, 1);
    assert_eq!(stats.misses, 2);
    assert_eq!(stats.catalog_invalidations, 1);
}

#[test]
fn explain_retrieve_aql_uses_query_cache() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_cell(
        CellId(1),
        b"scope=project:investments\nstatus=ready\nalpha budget".to_vec(),
    )
    .unwrap();
    let readable = view(scope_id("project:investments"));
    let query = r#"EXPLAIN RETRIEVE CONTEXT FOR TASK "budget" IN BRAIN investment_projects
WHERE space = project:investments AND status = "ready" LIMIT 10 CANDIDATES;"#;

    let first = db.explain_retrieve_aql(query, &readable).unwrap();
    let second = db.explain_retrieve_aql(query, &readable).unwrap();
    assert_eq!(first.bitmap_ops, second.bitmap_ops);
    assert_eq!(second.candidate_counts.after_bitmap, 1);
    assert_eq!(db.aql_query_cache_stats().unwrap().hits, 1);
}

fn view(scope: ScopeId) -> AgentView {
    AgentView {
        agent_id: AgentId(1),
        label: None,
        readable_brains: BTreeSet::from([BrainId(1)]),
        readable_scopes: BTreeSet::from([scope]),
        writable_scopes: BTreeSet::new(),
        allowed_modes: BTreeSet::from([RetrievalMode::Balanced]),
        allowed_memory_types: BTreeSet::from([MemoryType::Decision]),
        max_context_budget_tokens: 1_000,
        default_context_budget_tokens: 400,
        max_candidate_limit: 100,
        default_candidate_limit: 20,
        min_required_confidence_q16: Q16_ZERO,
        max_ttl_seconds: Some(3_600),
        allow_remember: false,
        allow_verify_fact: false,
        allow_audit_mode: false,
        require_citations_by_default: false,
        private_scope: None,
    }
}
