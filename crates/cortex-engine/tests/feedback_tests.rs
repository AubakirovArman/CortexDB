use cortex_aql::{AgentId, AgentView, BrainId, MemoryType, RetrievalMode, Q16_ZERO};
use cortex_core::CellId;
use cortex_engine::feedback::ContextFeedback;
use cortex_engine::{scope_id, Database};
use std::collections::BTreeSet;

#[test]
fn record_context_feedback_writes_durable_feedback_cell() {
    let dir = tempfile::tempdir().unwrap();
    let feedback_id = {
        let mut db = Database::open(dir.path()).unwrap();
        let stored = db
            .record_context_feedback(
                AgentId(7),
                ContextFeedback {
                    source_cell_id: CellId(42),
                    useful: true,
                    note: Some("good\ncontext".to_owned()),
                },
            )
            .unwrap();
        let payload = db.get_latest_cell(stored.cell_id).unwrap();
        let text = String::from_utf8_lossy(&payload);
        assert!(text.contains("scope=agent:7"));
        assert!(text.contains("type=feedback"));
        assert!(text.contains("source=cell:42"));
        assert!(text.contains("useful=true"));
        assert!(text.contains("note=good context"));
        stored.cell_id
    };

    let db = Database::open(dir.path()).unwrap();
    assert!(db.get_latest_cell(feedback_id).is_some());
}

#[test]
fn feedback_cells_are_queryable_by_scope_and_type() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    let stored = db
        .record_context_feedback(
            AgentId(7),
            ContextFeedback {
                source_cell_id: CellId(42),
                useful: false,
                note: None,
            },
        )
        .unwrap();

    let cells = db
        .retrieve_aql(
            r#"RETRIEVE CONTEXT FOR TASK "feedback" IN BRAIN investment_projects
WHERE scope = agent:7 AND type = "feedback" LIMIT 10 CANDIDATES;"#,
            &view(),
        )
        .unwrap();
    assert_eq!(cells.len(), 1);
    assert_eq!(cells[0].cell_id, stored.cell_id);
}

#[test]
fn feedback_scores_aggregate_useful_and_not_useful_votes() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.record_context_feedback(
        AgentId(7),
        ContextFeedback {
            source_cell_id: CellId(42),
            useful: true,
            note: None,
        },
    )
    .unwrap();
    db.record_context_feedback(
        AgentId(7),
        ContextFeedback {
            source_cell_id: CellId(42),
            useful: false,
            note: None,
        },
    )
    .unwrap();
    db.record_context_feedback(
        AgentId(7),
        ContextFeedback {
            source_cell_id: CellId(43),
            useful: true,
            note: None,
        },
    )
    .unwrap();

    let scores = db.feedback_scores();
    assert_eq!(scores.get(&CellId(42)), Some(&0));
    assert_eq!(scores.get(&CellId(43)), Some(&1));
}

fn view() -> AgentView {
    AgentView {
        agent_id: AgentId(7),
        label: None,
        readable_brains: BTreeSet::from([BrainId(1)]),
        readable_scopes: BTreeSet::from([scope_id("agent:7")]),
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
