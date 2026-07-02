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
fn feedback_cell_ids_preserve_max_documented_agent_slot() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    let low = db
        .record_context_feedback(AgentId(7), feedback(CellId(42), true))
        .unwrap();
    let high = db
        .record_context_feedback(AgentId(0x0fff_ffff), feedback(CellId(43), true))
        .unwrap();

    assert_eq!(encoded_agent_slot(low.cell_id), 7);
    assert_eq!(encoded_agent_slot(high.cell_id), 0x0fff_ffff);
}

#[test]
fn feedback_cell_ids_reject_agent_slot_overflow() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();

    let error = db
        .record_context_feedback(AgentId(0x1000_0007), feedback(CellId(42), true))
        .unwrap_err();
    assert!(matches!(
        error,
        cortex_engine::EngineError::StorageInvariant(_)
    ));
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

#[test]
fn feedback_index_tracks_patch_tombstone_checkpoint_and_reopen() {
    let dir = tempfile::tempdir().unwrap();
    {
        let mut db = Database::open(dir.path()).unwrap();
        db.put_knowledge_cell(CellId(10), feedback_cell(CellId(42), true, 100))
            .unwrap();
        db.put_knowledge_cell(CellId(11), feedback_cell(CellId(43), true, 100))
            .unwrap();

        assert_eq!(db.feedback_scores().get(&CellId(42)), Some(&1));
        assert_eq!(db.feedback_scores().get(&CellId(43)), Some(&1));

        db.patch_cell(
            CellId(10),
            feedback_cell(CellId(42), false, 100).encode_payload(),
        )
        .unwrap();
        db.tombstone_cell(CellId(11)).unwrap();

        let scores = db.feedback_scores();
        assert_eq!(scores.get(&CellId(42)), Some(&-1));
        assert_eq!(scores.get(&CellId(43)), None);

        db.checkpoint().unwrap();
        let scores_after_checkpoint = db.feedback_scores();
        assert_eq!(scores_after_checkpoint.get(&CellId(42)), Some(&-1));
        assert_eq!(scores_after_checkpoint.get(&CellId(43)), None);
    }

    let reopened = Database::open(dir.path()).unwrap();
    let scores_after_reopen = reopened.feedback_scores();
    assert_eq!(scores_after_reopen.get(&CellId(42)), Some(&-1));
    assert_eq!(scores_after_reopen.get(&CellId(43)), None);
}

#[test]
fn record_context_feedback_probes_past_occupied_cell_id() {
    // Regression for B1.1: feedback allocation must probe for a free id like
    // session allocation, instead of blindly writing `current_seq + 1` and
    // silently overwriting an existing feedback cell.
    const FEEDBACK_CELL_NAMESPACE: u64 = 0x9000_0000_0000_0000;
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();

    // Occupying a cell advances current_seq by 1, so occupying the id with
    // sequence (current_seq + 2) makes the next feedback candidate
    // (current_seq + 1 after the occupying write) land exactly on it.
    let slot: u64 = 7;
    let seq = db.current_seq().0;
    let collide_id = CellId(FEEDBACK_CELL_NAMESPACE | (slot << 32) | (seq + 2));
    db.put_knowledge_cell(collide_id, feedback_cell(CellId(99), true, 100))
        .unwrap();
    // The occupying write advanced current_seq so the next candidate == collide_id.
    assert_eq!(db.current_seq().0 + 1, seq + 2);

    let stored = db
        .record_context_feedback(AgentId(7), feedback(CellId(42), true))
        .unwrap();

    // Must have probed PAST the occupied id, not reused (and overwritten) it.
    assert_ne!(stored.cell_id, collide_id);
    // The occupied cell is intact (its source_cell_id=99 body survives).
    let occupied = db.get_latest_cell(collide_id).unwrap();
    assert!(String::from_utf8_lossy(&occupied).contains("source_cell_id=99"));
    // The new feedback cell is the real feedback for source cell 42.
    let new_cell = db.get_latest_cell(stored.cell_id).unwrap();
    assert!(String::from_utf8_lossy(&new_cell).contains("source=cell:42"));
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

fn encoded_agent_slot(cell_id: CellId) -> u64 {
    (cell_id.0 >> 32) & 0x0fff_ffff
}
