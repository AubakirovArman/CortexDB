use std::collections::BTreeSet;

use cortex_aql::{AgentId, AgentView, BrainId, MemoryType, RetrievalMode, Q16_ZERO};
use cortex_core::CellId;
use cortex_engine::{scope_id, AnnSearchPath, Database, SearchLimit, MIN_ANN_RECALL_Q16};

#[test]
fn database_ann_evaluation_reports_recall_against_exact_baseline() {
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
    db.put_cell(
        CellId(3),
        b"scope=tenant:private\nstatus=ready\nvector=0,11\n\nhidden".to_vec(),
    )
    .unwrap();
    db.checkpoint().unwrap();

    let report = db
        .evaluate_vector_ann(&[0, 10], &view("project:investments"), SearchLimit(2))
        .unwrap()
        .expect("persisted checkpoint should be evaluable");

    assert_eq!(report.search.path, AnnSearchPath::HnswGraph);
    assert_eq!(report.exact_top_k, vec![2, 1]);
    assert_eq!(report.ann_top_k, vec![2, 1]);
    assert_eq!(report.overlap_count, 2);
    assert_eq!(report.recall_q16, 65_535);
    assert_eq!(report.search.recall_q16, Some(65_535));
    assert_eq!(report.search.min_recall_q16, Some(MIN_ANN_RECALL_Q16));
}

#[test]
fn database_ann_evaluation_waits_for_persisted_snapshot() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_cell(
        CellId(1),
        b"scope=project:investments\nstatus=ready\nvector=10,0\n\nalpha".to_vec(),
    )
    .unwrap();

    assert!(db
        .evaluate_vector_ann(&[10, 0], &view("project:investments"), SearchLimit(1))
        .unwrap()
        .is_none());

    db.checkpoint().unwrap();
    assert!(db
        .evaluate_vector_ann(&[10, 0], &view("project:investments"), SearchLimit(1))
        .unwrap()
        .is_some());

    db.patch_cell(
        CellId(1),
        b"scope=project:investments\nstatus=ready\nvector=0,10\n\nfresh".to_vec(),
    )
    .unwrap();
    assert!(db
        .evaluate_vector_ann(&[0, 10], &view("project:investments"), SearchLimit(1))
        .unwrap()
        .is_none());
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
