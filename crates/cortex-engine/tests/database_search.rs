use std::collections::BTreeSet;

use cortex_aql::{AgentId, AgentView, BrainId, MemoryType, RetrievalMode, Q16_ZERO};
use cortex_core::CellId;
use cortex_engine::{scope_id, Database, SearchLimit};

#[test]
fn database_keyword_search_returns_visible_cells() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_cell(
        CellId(1),
        b"scope=project:investments\nstatus=ready\nalpha budget approved".to_vec(),
    )
    .unwrap();
    db.put_cell(
        CellId(2),
        b"scope=tenant:private\nstatus=ready\nalpha budget hidden".to_vec(),
    )
    .unwrap();

    let results = db
        .search_keyword("budget", &view("project:investments"), SearchLimit(10))
        .unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].cell_id, CellId(1));
    assert!(String::from_utf8_lossy(&results[0].payload).contains("approved"));
}

#[test]
fn database_keyword_search_survives_checkpoint_restart() {
    let dir = tempfile::tempdir().unwrap();
    {
        let mut db = Database::open(dir.path()).unwrap();
        db.put_cell(
            CellId(u64::MAX),
            b"scope=project:investments\nstatus=ready\nlarge budget".to_vec(),
        )
        .unwrap();
        db.checkpoint().unwrap();
    }

    let db = Database::open(dir.path()).unwrap();
    let results = db
        .search_keyword("budget", &view("project:investments"), SearchLimit(10))
        .unwrap();
    assert_eq!(results[0].cell_id, CellId(u64::MAX));
}

#[test]
fn database_keyword_search_uses_body_terms_not_header_terms() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_cell(
        CellId(1),
        b"scope=project:investments\nstatus=ready\n\nbody budget".to_vec(),
    )
    .unwrap();

    assert!(db
        .search_keyword("project", &view("project:investments"), SearchLimit(10))
        .unwrap()
        .is_empty());
    assert_eq!(
        db.search_keyword("budget", &view("project:investments"), SearchLimit(10))
            .unwrap()[0]
            .cell_id,
        CellId(1)
    );
}

fn view(scope: &str) -> AgentView {
    AgentView {
        agent_id: AgentId(1),
        label: None,
        readable_brains: BTreeSet::from([BrainId(1)]),
        readable_scopes: BTreeSet::from([scope_id(scope)]),
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
