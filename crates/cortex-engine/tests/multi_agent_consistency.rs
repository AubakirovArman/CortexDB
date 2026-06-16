use std::collections::BTreeSet;

use cortex_aql::{AgentId, AgentView, BrainId, MemoryType, RetrievalMode, Q16_ZERO};
use cortex_core::{CellId, CommitSeq};
use cortex_engine::{
    classify_memory_visibility, scope_id, AgentHandoffRequest, AgentTransactionConflictKind,
    AgentTransactionOptions, AgentTransactionOutcome, AgentTransactionRequest, Database,
    DatabaseOptions, EngineError, MemoryConsistencyLevel, WriteBatch,
};

fn options() -> DatabaseOptions {
    DatabaseOptions {
        agent_transactions: AgentTransactionOptions { enabled: true },
        ..DatabaseOptions::default()
    }
}

fn view(agent_id: u64, readable: &[&str], writable: &[&str], private: Option<&str>) -> AgentView {
    AgentView {
        agent_id: AgentId(agent_id),
        label: Some(format!("agent-{agent_id}")),
        readable_brains: BTreeSet::from([BrainId(1)]),
        readable_scopes: readable.iter().map(|scope| scope_id(scope)).collect(),
        writable_scopes: writable.iter().map(|scope| scope_id(scope)).collect(),
        allowed_modes: BTreeSet::from([RetrievalMode::Balanced]),
        allowed_memory_types: BTreeSet::from([MemoryType::Decision]),
        max_context_budget_tokens: 4_000,
        default_context_budget_tokens: 1_000,
        max_candidate_limit: 32,
        default_candidate_limit: 20,
        min_required_confidence_q16: Q16_ZERO,
        max_ttl_seconds: Some(3_600),
        allow_remember: true,
        allow_verify_fact: true,
        allow_audit_mode: false,
        require_citations_by_default: false,
        private_scope: private.map(scope_id),
    }
}

fn payload(scope: &str, body: &str) -> Vec<u8> {
    format!("scope={scope}\nstatus=ready\ntype=fact\n\n{body}").into_bytes()
}

fn request(
    agent_id: u64,
    scope: &str,
    base_seq: CommitSeq,
    batch: WriteBatch,
) -> AgentTransactionRequest {
    AgentTransactionRequest {
        agent_id: AgentId(agent_id),
        scope: scope.to_owned(),
        base_seq,
        batch,
        idempotency_key: None,
    }
}

fn retrieve_ids(db: &Database, view: &AgentView, query: &str) -> Vec<CellId> {
    db.retrieve_aql(query, view)
        .unwrap()
        .into_iter()
        .map(|cell| cell.cell_id)
        .collect()
}

#[test]
fn private_memory_is_read_your_writes_and_hidden_from_other_agent() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open_with_options(dir.path(), options()).unwrap();
    let owner = view(1, &["agent:one"], &["agent:one"], Some("agent:one"));
    let other = view(2, &["shared:project"], &[], Some("agent:two"));
    let committed = db
        .commit_agent_transaction(
            &owner,
            request(
                1,
                "agent:one",
                db.current_seq(),
                WriteBatch::new().put_cell(CellId(1), payload("agent:one", "private alpha")),
            ),
        )
        .unwrap()
        .committed_seq
        .unwrap();

    let owner_visibility = classify_memory_visibility(&owner, AgentId(1), "agent:one", committed);
    assert_eq!(
        owner_visibility.level,
        MemoryConsistencyLevel::PrivateReadYourWrites
    );
    assert!(owner_visibility.readable);
    assert_eq!(owner_visibility.visible_after_seq, committed);

    let other_visibility = classify_memory_visibility(&other, AgentId(1), "agent:one", committed);
    assert!(!other_visibility.readable);

    assert_eq!(retrieve_ids(&db, &owner, alpha_query()), vec![CellId(1)]);
    assert!(retrieve_ids(&db, &other, alpha_query()).is_empty());
}

#[test]
fn shared_memory_is_immediately_visible_after_commit_sequence() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open_with_options(dir.path(), options()).unwrap();
    let writer = view(1, &["shared:project"], &["shared:project"], None);
    let reader = view(2, &["shared:project"], &[], None);

    let report = db
        .commit_agent_transaction(
            &writer,
            request(
                1,
                "shared:project",
                db.current_seq(),
                WriteBatch::new().put_cell(CellId(2), payload("shared:project", "shared beta")),
            ),
        )
        .unwrap();
    let committed = report.committed_seq.unwrap();
    let visibility = classify_memory_visibility(&reader, AgentId(1), "shared:project", committed);

    assert_eq!(visibility.level, MemoryConsistencyLevel::SharedImmediate);
    assert!(visibility.readable);
    assert!(!visibility.writable);
    assert_eq!(visibility.visible_after_seq, committed);
    assert_eq!(retrieve_ids(&db, &reader, beta_query()), vec![CellId(2)]);
}

#[test]
fn shared_same_cell_writes_conflict_from_stale_base_sequence() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open_with_options(dir.path(), options()).unwrap();
    let first = view(1, &["shared:project"], &["shared:project"], None);
    let second = view(2, &["shared:project"], &["shared:project"], None);
    let base_seq = db.current_seq();

    let first_report = db
        .commit_agent_transaction(
            &first,
            request(
                1,
                "shared:project",
                base_seq,
                WriteBatch::new().put_cell(CellId(3), payload("shared:project", "agent one")),
            ),
        )
        .unwrap();
    assert_eq!(first_report.outcome, AgentTransactionOutcome::Committed);

    let second_report = db
        .commit_agent_transaction(
            &second,
            request(
                2,
                "shared:project",
                base_seq,
                WriteBatch::new().put_cell(CellId(3), payload("shared:project", "agent two")),
            ),
        )
        .unwrap();

    assert_eq!(second_report.outcome, AgentTransactionOutcome::Conflict);
    assert_eq!(second_report.conflicts.len(), 1);
    assert_eq!(
        second_report.conflicts[0].kind,
        AgentTransactionConflictKind::StaleCell
    );
}

#[test]
fn sequenced_handoff_requires_target_visibility_and_stable_pack_seq() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open_with_options(dir.path(), options()).unwrap();
    let source = view(1, &["shared:project"], &["shared:project"], None);
    let target = view(2, &["shared:project"], &[], None);
    let blocked_target = view(3, &["agent:three"], &[], Some("agent:three"));

    let committed = db
        .commit_agent_transaction(
            &source,
            request(
                1,
                "shared:project",
                db.current_seq(),
                WriteBatch::new().put_cell(CellId(4), payload("shared:project", "handoff gamma")),
            ),
        )
        .unwrap()
        .committed_seq
        .unwrap();

    let handoff = db
        .plan_agent_handoff(
            &source,
            &target,
            AgentHandoffRequest {
                source_agent_id: AgentId(1),
                target_agent_id: AgentId(2),
                scope: "shared:project".to_owned(),
                pack_hash: "ctxpack:v1:gamma".to_owned(),
                pack_seq: committed,
                required_after_seq: CommitSeq(0),
                idempotency_key: Some("handoff-1-2-gamma".to_owned()),
            },
        )
        .unwrap();

    assert_eq!(handoff.level, MemoryConsistencyLevel::SharedSequenced);
    assert_eq!(handoff.visible_after_seq, committed);
    assert!(handoff.target_can_read);

    let blocked = db
        .plan_agent_handoff(
            &source,
            &blocked_target,
            AgentHandoffRequest {
                source_agent_id: AgentId(1),
                target_agent_id: AgentId(3),
                scope: "shared:project".to_owned(),
                pack_hash: "ctxpack:v1:gamma".to_owned(),
                pack_seq: committed,
                required_after_seq: committed,
                idempotency_key: None,
            },
        )
        .unwrap_err();
    assert_eq!(blocked.code().as_str(), "permission_denied");

    let future_seq = db
        .plan_agent_handoff(
            &source,
            &target,
            AgentHandoffRequest {
                source_agent_id: AgentId(1),
                target_agent_id: AgentId(2),
                scope: "shared:project".to_owned(),
                pack_hash: "ctxpack:v1:future".to_owned(),
                pack_seq: CommitSeq(committed.0 + 1),
                required_after_seq: committed,
                idempotency_key: None,
            },
        )
        .unwrap_err();
    assert!(matches!(future_seq, EngineError::InvalidAgentSession(_)));
}

fn alpha_query() -> &'static str {
    r#"RETRIEVE CONTEXT FOR TASK "alpha" IN BRAIN investment_projects
WHERE status = "ready" LIMIT 10 CANDIDATES;"#
}

fn beta_query() -> &'static str {
    r#"RETRIEVE CONTEXT FOR TASK "beta" IN BRAIN investment_projects
WHERE status = "ready" LIMIT 10 CANDIDATES;"#
}
