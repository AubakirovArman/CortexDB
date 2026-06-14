use cortex_aql::AgentId;
use cortex_core::{CellId, KnowledgeCell, KnowledgeCellMetadata, KnowledgeCellType};
use cortex_engine::feedback::{ContextFeedback, FEEDBACK_FULL_VOTE_BONUS};
use cortex_engine::Database;

#[test]
fn feedback_scores_for_cells_only_materializes_requested_targets() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.record_context_feedback(AgentId(7), feedback(CellId(42), true))
        .unwrap();
    db.record_context_feedback(AgentId(7), feedback(CellId(99), true))
        .unwrap();

    let scores = db.feedback_scores_for_cells_at([CellId(42)], u64::MAX);

    assert_eq!(scores.len(), 1);
    assert_eq!(scores.get(&CellId(42)), Some(&0));
    assert!(!scores.contains_key(&CellId(99)));
}

#[test]
fn feedback_score_lookup_uses_indexed_target_records() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_knowledge_cell(CellId(10), feedback_cell(CellId(42), true, 100))
        .unwrap();
    db.put_knowledge_cell(CellId(11), feedback_cell(CellId(42), true, 100))
        .unwrap();
    db.put_knowledge_cell(CellId(12), feedback_cell(CellId(99), false, 100))
        .unwrap();

    assert_eq!(
        db.feedback_score_for_cell_at(CellId(42), 100),
        FEEDBACK_FULL_VOTE_BONUS * 2
    );
    assert_eq!(
        db.feedback_score_for_cell_at(CellId(99), 100),
        -FEEDBACK_FULL_VOTE_BONUS
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
