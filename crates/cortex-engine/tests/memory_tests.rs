use std::collections::BTreeSet;

use cortex_aql::{AgentId, AgentView, BrainId, MemoryType, RetrievalMode, Q16_ONE, Q16_ZERO};
use cortex_core::{CellId, KnowledgeCell, KnowledgeCellMetadata, KnowledgeCellType};
use cortex_engine::{scope_id, Database};

#[test]
fn expired_memory_cells_reports_only_due_memory() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_knowledge_cell(CellId(1), memory_cell(100, Some(10)))
        .unwrap();
    db.put_knowledge_cell(CellId(2), memory_cell(100, Some(200)))
        .unwrap();
    db.put_knowledge_cell(CellId(3), memory_cell(100, None))
        .unwrap();

    let expired = db.expired_memory_cells(111);
    assert_eq!(expired.len(), 1);
    assert_eq!(expired[0].cell_id, CellId(1));
    assert_eq!(expired[0].expired_at_unix_seconds, 110);
}

#[test]
fn expire_memory_cells_tombstones_through_wal_and_survives_restart() {
    let dir = tempfile::tempdir().unwrap();
    {
        let mut db = Database::open(dir.path()).unwrap();
        db.put_knowledge_cell(CellId(1), memory_cell(100, Some(10)))
            .unwrap();
        let expired = db.expire_memory_cells(111).unwrap();
        assert_eq!(expired[0].cell_id, CellId(1));
        assert!(db.get_latest_cell(CellId(1)).is_none());
    }

    let db = Database::open(dir.path()).unwrap();
    assert!(db.get_latest_cell(CellId(1)).is_none());
}

#[test]
fn expire_memory_cells_excludes_expired_memory_from_aql_retrieve() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_knowledge_cell(CellId(1), memory_cell(100, Some(10)))
        .unwrap();
    db.put_knowledge_cell(CellId(2), memory_cell(100, Some(200)))
        .unwrap();

    db.expire_memory_cells(111).unwrap();

    let cells = db
        .retrieve_aql(
            r#"RETRIEVE CONTEXT FOR TASK "memory" IN BRAIN investment_projects
WHERE scope = project:investments AND type = "memory" AND memory_type = "decision"
LIMIT 10 CANDIDATES;"#,
            &view(),
        )
        .unwrap();
    assert_eq!(cells.len(), 1);
    assert_eq!(cells[0].cell_id, CellId(2));
}

#[test]
fn memory_decay_scores_are_fixed_point_and_ttl_aware() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_knowledge_cell(CellId(1), memory_cell(100, Some(100)))
        .unwrap();
    db.put_knowledge_cell(CellId(2), memory_cell(100, None))
        .unwrap();

    let scores = db.memory_decay_scores(150);
    assert_eq!(scores.len(), 2);
    let expiring = scores
        .iter()
        .find(|score| score.cell_id == CellId(1))
        .unwrap();
    let permanent = scores
        .iter()
        .find(|score| score.cell_id == CellId(2))
        .unwrap();
    assert_eq!(expiring.freshness_q16, 32_768);
    assert_eq!(expiring.age_seconds, Some(50));
    assert_eq!(expiring.ttl_seconds, Some(100));
    assert_eq!(permanent.freshness_q16, Q16_ONE);

    let expired = db
        .memory_decay_scores(201)
        .into_iter()
        .find(|score| score.cell_id == CellId(1))
        .unwrap();
    assert_eq!(expired.freshness_q16, Q16_ZERO);
}

fn memory_cell(created: u64, ttl: Option<u64>) -> KnowledgeCell {
    KnowledgeCell::new(
        KnowledgeCellMetadata {
            scope: "project:investments".to_owned(),
            status: "ready".to_owned(),
            cell_type: KnowledgeCellType::Memory,
            memory_type: Some("decision".to_owned()),
            ttl_seconds: ttl,
            created_unix_seconds: Some(created),
            source_trust_q16: None,
            source: Some("test".to_owned()),
        },
        "memory payload",
    )
}

fn view() -> AgentView {
    AgentView {
        agent_id: AgentId(1),
        label: None,
        readable_brains: BTreeSet::from([BrainId(1)]),
        readable_scopes: BTreeSet::from([scope_id("project:investments")]),
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
