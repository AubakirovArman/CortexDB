use cortex_aql::{AgentId, AgentView, BrainId, MemoryType, RetrievalMode, Q16_ZERO};
use cortex_core::{CellId, KnowledgeCell, KnowledgeCellMetadata, KnowledgeCellType};
use cortex_engine::feedback::{
    ContextFeedback, FEEDBACK_DECAY_WINDOW_SECONDS, FEEDBACK_FULL_VOTE_BONUS,
};
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

#[test]
fn feedback_stats_reports_totals_and_per_cell_breakdown() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.record_context_feedback(AgentId(7), feedback(CellId(42), true))
        .unwrap();
    db.record_context_feedback(AgentId(7), feedback(CellId(42), false))
        .unwrap();
    db.record_context_feedback(AgentId(7), feedback(CellId(43), true))
        .unwrap();

    let stats = db.feedback_stats();
    assert_eq!(stats.total, 3);
    assert_eq!(stats.useful, 2);
    assert_eq!(stats.not_useful, 1);
    assert_eq!(stats.by_source_cell[&CellId(42)].score, 0);
    assert_eq!(stats.by_source_cell[&CellId(42)].useful, 1);
    assert_eq!(stats.by_source_cell[&CellId(42)].not_useful, 1);
    assert_eq!(stats.by_source_cell[&CellId(43)].score, 1);
}

#[test]
fn feedback_scores_at_decay_with_fixed_window() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_knowledge_cell(CellId(10), feedback_cell(CellId(42), true, 100))
        .unwrap();
    db.put_knowledge_cell(CellId(11), feedback_cell(CellId(43), false, 100))
        .unwrap();

    let half_window = 100 + (FEEDBACK_DECAY_WINDOW_SECONDS / 2);
    let scores = db.feedback_scores_at(half_window);
    assert_eq!(
        scores.get(&CellId(42)),
        Some(&(FEEDBACK_FULL_VOTE_BONUS / 2))
    );
    assert_eq!(
        scores.get(&CellId(43)),
        Some(&(-(FEEDBACK_FULL_VOTE_BONUS / 2)))
    );

    let expired = db.feedback_scores_at(100 + FEEDBACK_DECAY_WINDOW_SECONDS);
    assert_eq!(expired.get(&CellId(42)), Some(&0));
    assert_eq!(expired.get(&CellId(43)), Some(&0));
}

#[test]
fn feedback_score_report_explains_raw_and_decayed_contribution() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_knowledge_cell(CellId(10), feedback_cell(CellId(42), true, 100))
        .unwrap();
    db.put_knowledge_cell(CellId(11), feedback_cell(CellId(42), false, 100))
        .unwrap();
    db.put_knowledge_cell(CellId(12), feedback_cell(CellId(42), true, 100))
        .unwrap();

    let report = db.feedback_score_report_at(100);
    assert_eq!(report.len(), 1);
    assert_eq!(report[0].source_cell_id, CellId(42));
    assert_eq!(report[0].useful, 2);
    assert_eq!(report[0].not_useful, 1);
    assert_eq!(report[0].raw_score, 1);
    assert_eq!(report[0].decayed_score, FEEDBACK_FULL_VOTE_BONUS);
    assert_eq!(
        report[0].decay_window_seconds,
        FEEDBACK_DECAY_WINDOW_SECONDS
    );
}

fn feedback(source_cell_id: CellId, useful: bool) -> ContextFeedback {
    ContextFeedback {
        source_cell_id,
        useful,
        note: None,
    }
}

fn feedback_cell(source_cell_id: CellId, useful: bool, created: u64) -> KnowledgeCell {
    KnowledgeCell::new(
        KnowledgeCellMetadata {
            scope: "agent:7".to_owned(),
            status: "ready".to_owned(),
            cell_type: KnowledgeCellType::Feedback,
            memory_type: None,
            ttl_seconds: None,
            created_unix_seconds: Some(created),
            source_trust_q16: None,
            source: Some(format!("cell:{}", source_cell_id.0)),
        },
        format!("source_cell_id={}\nuseful={}\n", source_cell_id.0, useful),
    )
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
