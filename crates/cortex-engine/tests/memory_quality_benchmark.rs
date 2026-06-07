use std::collections::BTreeSet;

use cortex_aql::{AgentId, AgentView, BrainId, MemoryType, RetrievalMode, Q16_ONE, Q16_ZERO};
use cortex_core::{CellId, KnowledgeCell, KnowledgeCellMetadata, KnowledgeCellType};
use cortex_engine::feedback::{ContextFeedback, FeedbackStats};
use cortex_engine::{scope_id, ContextPackOptions, Database};

#[test]
fn memory_quality_update_handling_prefers_latest_payload() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_knowledge_cell(
        CellId(100),
        memory_cell(
            "project:investments",
            "decision",
            100,
            None,
            "Prefer preliminary budget evidence",
        ),
    )
    .unwrap();
    db.put_knowledge_cell(
        CellId(100),
        memory_cell(
            "project:investments",
            "decision",
            200,
            None,
            "Prefer audited budget evidence",
        ),
    )
    .unwrap();

    let query = memory_query("project:investments");
    let cells = db
        .retrieve_aql(&query, &view(&["project:investments"]))
        .unwrap();
    assert_eq!(cells.len(), 1);
    let payload = String::from_utf8_lossy(&cells[0].payload);
    assert!(payload.contains("Prefer audited budget evidence"));
    assert!(!payload.contains("Prefer preliminary budget evidence"));
}

#[test]
fn memory_quality_stale_memory_detection_expires_and_scores_memory() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_knowledge_cell(
        CellId(101),
        memory_cell(
            "project:investments",
            "decision",
            1_000,
            Some(60),
            "Temporary comparison rule",
        ),
    )
    .unwrap();
    db.put_knowledge_cell(
        CellId(102),
        memory_cell(
            "project:investments",
            "decision",
            1_000,
            None,
            "Permanent citation rule",
        ),
    )
    .unwrap();

    let stale = db.expired_memory_cells(1_061);
    assert_eq!(stale.len(), 1);
    assert_eq!(stale[0].cell_id, CellId(101));
    assert_eq!(stale[0].expired_at_unix_seconds, 1_060);

    let scores = db.memory_decay_scores(1_030);
    let temporary = scores
        .iter()
        .find(|score| score.cell_id == CellId(101))
        .unwrap();
    let permanent = scores
        .iter()
        .find(|score| score.cell_id == CellId(102))
        .unwrap();
    assert_eq!(temporary.freshness_q16, 32_768);
    assert_eq!(permanent.freshness_q16, Q16_ONE);

    db.expire_memory_cells(1_061).unwrap();
    let query = memory_query("project:investments");
    let cells = db
        .retrieve_aql(&query, &view(&["project:investments"]))
        .unwrap();
    assert_eq!(cells.len(), 1);
    assert_eq!(cells[0].cell_id, CellId(102));
}

#[test]
fn memory_quality_preference_retrieval_uses_feedback_signal() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_knowledge_cell(
        CellId(201),
        memory_cell(
            "project:investments",
            "decision",
            100,
            None,
            "Prefer oldest available evidence",
        ),
    )
    .unwrap();
    db.put_knowledge_cell(
        CellId(202),
        memory_cell(
            "project:investments",
            "decision",
            100,
            None,
            "Prefer cited budget evidence",
        ),
    )
    .unwrap();
    db.record_context_feedback(
        AgentId(7),
        ContextFeedback {
            source_cell_id: CellId(202),
            useful: true,
            note: Some("best memory preference".to_owned()),
        },
    )
    .unwrap();

    let query = memory_query("project:investments");
    let pack = db
        .context_pack_from_aql(
            &query,
            &view(&["project:investments"]),
            ContextPackOptions::default(),
        )
        .unwrap();
    assert_eq!(pack.cells[0].cell_id, CellId(202));

    let stats: FeedbackStats = db.feedback_stats();
    assert_eq!(stats.by_source_cell[&CellId(202)].score, 1);
}

#[test]
fn memory_quality_temporal_changes_preserve_snapshot_visibility() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_knowledge_cell(
        CellId(301),
        memory_cell(
            "project:investments",
            "decision",
            100,
            None,
            "Use Q1 investment assumptions",
        ),
    )
    .unwrap();
    let old_txn = db.read_txn();

    db.put_knowledge_cell(
        CellId(301),
        memory_cell(
            "project:investments",
            "decision",
            200,
            None,
            "Use Q2 investment assumptions",
        ),
    )
    .unwrap();

    let old_payload =
        String::from_utf8_lossy(&db.get_cell(old_txn, CellId(301)).unwrap()).to_string();
    let latest_payload =
        String::from_utf8_lossy(&db.get_latest_cell(CellId(301)).unwrap()).to_string();
    assert!(old_payload.contains("Use Q1 investment assumptions"));
    assert!(latest_payload.contains("Use Q2 investment assumptions"));
}

fn memory_cell(
    scope: &str,
    memory_type: &str,
    created: u64,
    ttl: Option<u64>,
    body: &str,
) -> KnowledgeCell {
    KnowledgeCell::new(
        KnowledgeCellMetadata {
            scope: scope.to_owned(),
            status: "ready".to_owned(),
            cell_type: KnowledgeCellType::Memory,
            memory_type: Some(memory_type.to_owned()),
            ttl_seconds: ttl,
            created_unix_seconds: Some(created),
            source_trust_q16: None,
            source: Some("memory-quality-benchmark".to_owned()),
        },
        body,
    )
}

fn memory_query(scope: &str) -> String {
    format!(
        r#"RETRIEVE CONTEXT FOR TASK "memory quality" IN BRAIN investment_projects
WHERE scope = {scope} AND type = "memory" AND memory_type = "decision"
LIMIT 10 CANDIDATES;"#
    )
}

fn view(scopes: &[&str]) -> AgentView {
    AgentView {
        agent_id: AgentId(7),
        label: None,
        readable_brains: BTreeSet::from([BrainId(1)]),
        readable_scopes: scopes.iter().map(|scope| scope_id(scope)).collect(),
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
