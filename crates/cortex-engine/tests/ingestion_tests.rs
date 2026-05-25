use cortex_aql::{AgentId, AgentView, BrainId, MemoryType, RetrievalMode, Q16_ZERO};
use cortex_core::{CellId, KnowledgeCell, KnowledgeCellMetadata, KnowledgeCellType};
use cortex_engine::{scope_id, Database};
use std::collections::BTreeSet;

#[test]
fn put_knowledge_cell_encodes_metadata_for_aql_retrieve() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    let cell = KnowledgeCell::new(
        KnowledgeCellMetadata {
            scope: "project:investments".to_owned(),
            status: "verified".to_owned(),
            cell_type: KnowledgeCellType::Fact,
            memory_type: None,
            source: Some("annual-report".to_owned()),
        },
        "Бюджет проекта ABC подтвержден",
    );
    db.put_knowledge_cell(CellId(77), cell).unwrap();

    let cells = db
        .retrieve_aql(
            r#"RETRIEVE CONTEXT FOR TASK "budget" IN BRAIN investment_projects
WHERE space = project:investments AND status = "verified" AND type = "fact" LIMIT 10 CANDIDATES;"#,
            &view(),
        )
        .unwrap();
    assert_eq!(cells.len(), 1);
    assert_eq!(cells[0].cell_id, CellId(77));
    assert!(String::from_utf8_lossy(&cells[0].payload).contains("source=annual-report"));
}

#[test]
fn knowledge_cell_metadata_sanitizes_header_lines() {
    let cell = KnowledgeCell::new(
        KnowledgeCellMetadata {
            scope: "tenant\nalpha".to_owned(),
            ..KnowledgeCellMetadata::default()
        },
        "payload",
    );
    let payload = String::from_utf8(cell.encode_payload()).unwrap();
    assert!(payload.contains("scope=tenant alpha"));
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
