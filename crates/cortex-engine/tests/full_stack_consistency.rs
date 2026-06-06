use std::collections::BTreeSet;

use cortex_aql::{AgentId, AgentView, BrainId, MemoryType, RetrievalMode, Q16_ZERO};
use cortex_core::{CellId, CommitSeq};
use cortex_engine::{scope_id, ContextPackAnomalyCode, ContextPackOptions, Database};

#[test]
fn full_stack_write_recover_query_pack_compact_and_repair_stays_consistent() {
    let dir = tempfile::tempdir().unwrap();
    {
        let mut db = Database::open(dir.path()).unwrap();
        db.put_cell(CellId(1), ready_project("doc-a", "alpha budget"))
            .unwrap();
        db.put_cell(CellId(2), ready_private("doc-private", "private budget"))
            .unwrap();
        assert_project_ready_context(&db, &[CellId(1)]);
        db.checkpoint().unwrap();
    }

    {
        let mut db = Database::open(dir.path()).unwrap();
        assert_eq!(db.current_seq(), CommitSeq(2));
        assert_eq!(db.validate_storage().unwrap().live_segments_checked, 1);
        assert_project_ready_context(&db, &[CellId(1)]);
        db.patch_cell(CellId(1), draft_project("doc-a", "alpha budget draft"))
            .unwrap();
        db.put_cell(CellId(3), ready_project("doc-c", "gamma budget"))
            .unwrap();
    }

    {
        let mut db = Database::open(dir.path()).unwrap();
        assert_eq!(db.current_seq(), CommitSeq(4));
        assert_project_ready_context(&db, &[CellId(3)]);
        db.compact().unwrap();
    }

    let repair = Database::repair_best_effort(dir.path()).unwrap();
    assert!(!repair.wal_truncated);

    let db = Database::open(dir.path()).unwrap();
    assert_eq!(db.current_seq(), CommitSeq(4));
    assert_eq!(db.storage_stats().unwrap().live_segments, 1);
    assert_project_ready_context(&db, &[CellId(3)]);
}

#[test]
fn context_pack_reports_policy_visible_citation_state_after_restart() {
    let dir = tempfile::tempdir().unwrap();
    {
        let mut db = Database::open(dir.path()).unwrap();
        db.put_cell(
            CellId(1),
            b"scope=project:investments\nstatus=ready\nbudget evidence without citation".to_vec(),
        )
        .unwrap();
    }

    let db = Database::open(dir.path()).unwrap();
    let pack = db
        .context_pack_from_aql(
            ready_query(),
            &view(true),
            ContextPackOptions {
                token_budget_tokens: 128,
                require_citations: false,
                ..ContextPackOptions::default()
            },
        )
        .unwrap();

    assert_eq!(pack.cells.len(), 1);
    assert_eq!(pack.answerability_q16, u16::MAX);
    assert_eq!(pack.anomalies.len(), 1);
    assert_eq!(
        pack.anomalies[0].code,
        ContextPackAnomalyCode::MissingCitation
    );
}

fn assert_project_ready_context(db: &Database, expected: &[CellId]) {
    let cells = db.retrieve_aql(ready_query(), &view(false)).unwrap();
    let cell_ids = cells.iter().map(|cell| cell.cell_id).collect::<Vec<_>>();
    assert_eq!(cell_ids, expected);

    let pack = db
        .context_pack_from_aql(
            ready_query(),
            &view(true),
            ContextPackOptions {
                token_budget_tokens: 256,
                require_citations: false,
                ..ContextPackOptions::default()
            },
        )
        .unwrap();
    let packed_ids = pack
        .cells
        .iter()
        .map(|cell| cell.cell_id)
        .collect::<Vec<_>>();
    assert_eq!(packed_ids, expected);
    assert!(pack.anomalies.is_empty());
}

fn ready_query() -> &'static str {
    r#"RETRIEVE CONTEXT FOR TASK "budget" IN BRAIN investment_projects
WHERE status = "ready" LIMIT 10 CANDIDATES;"#
}

fn ready_project(source: &str, body: &str) -> Vec<u8> {
    format!("scope=project:investments\nstatus=ready\nsource={source}\n{body}").into_bytes()
}

fn draft_project(source: &str, body: &str) -> Vec<u8> {
    format!("scope=project:investments\nstatus=draft\nsource={source}\n{body}").into_bytes()
}

fn ready_private(source: &str, body: &str) -> Vec<u8> {
    format!("scope=private\nstatus=ready\nsource={source}\n{body}").into_bytes()
}

fn view(require_citations: bool) -> AgentView {
    AgentView {
        agent_id: AgentId(1),
        label: Some("full-stack-test".to_owned()),
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
        require_citations_by_default: require_citations,
        private_scope: None,
    }
}
