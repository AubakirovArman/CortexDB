use std::collections::BTreeSet;

use cortex_aql::{AgentId, AgentView, BrainId, MemoryType, RetrievalMode, Q16_ZERO};
use cortex_core::{CellId, CommitSeq};
use cortex_engine::{
    scope_id, AgentTransactionConflictKind, AgentTransactionOptions, AgentTransactionOutcome,
    AgentTransactionRequest, Database, DatabaseOptions, EngineError, WriteBatch,
};

fn view(agent_id: u64, scope: &str) -> AgentView {
    let scope = scope_id(scope);
    AgentView {
        agent_id: AgentId(agent_id),
        label: Some(format!("agent-{agent_id}")),
        readable_brains: BTreeSet::from([BrainId(1)]),
        readable_scopes: BTreeSet::from([scope]),
        writable_scopes: BTreeSet::from([scope]),
        allowed_modes: BTreeSet::from([RetrievalMode::Balanced]),
        allowed_memory_types: BTreeSet::from([MemoryType::Decision]),
        max_context_budget_tokens: 4_000,
        default_context_budget_tokens: 1_000,
        max_candidate_limit: 32,
        default_candidate_limit: 20,
        min_required_confidence_q16: Q16_ZERO,
        max_ttl_seconds: None,
        allow_remember: true,
        allow_verify_fact: true,
        allow_audit_mode: false,
        require_citations_by_default: false,
        private_scope: Some(scope),
    }
}

fn options() -> DatabaseOptions {
    DatabaseOptions {
        agent_transactions: AgentTransactionOptions { enabled: true },
        ..DatabaseOptions::default()
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
        idempotency_key: Some(format!("agent-{agent_id}-{scope}-{}", base_seq.0)),
    }
}

#[test]
fn agent_transactions_require_feature_flag() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    let error = db
        .commit_agent_transaction(
            &view(1, "shared:project"),
            request(
                1,
                "shared:project",
                db.current_seq(),
                WriteBatch::new().put_cell(CellId(1), payload("shared:project", "a1")),
            ),
        )
        .unwrap_err();

    assert!(matches!(
        error,
        EngineError::FeatureDisabled("agent_transactions")
    ));
}

#[test]
fn concurrent_agent_transactions_conflict_on_stale_same_cell_write() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open_with_options(dir.path(), options()).unwrap();
    let base_seq = db.current_seq();

    let first = db
        .commit_agent_transaction(
            &view(1, "shared:project"),
            request(
                1,
                "shared:project",
                base_seq,
                WriteBatch::new().put_cell(CellId(10), payload("shared:project", "agent 1")),
            ),
        )
        .unwrap();
    assert_eq!(first.outcome, AgentTransactionOutcome::Committed);
    assert_eq!(first.committed_seq, Some(CommitSeq(1)));

    let second = db
        .commit_agent_transaction(
            &view(2, "shared:project"),
            request(
                2,
                "shared:project",
                base_seq,
                WriteBatch::new().put_cell(CellId(10), payload("shared:project", "agent 2")),
            ),
        )
        .unwrap();

    assert_eq!(second.outcome, AgentTransactionOutcome::Conflict);
    assert_eq!(second.committed_seq, None);
    assert_eq!(second.conflicts.len(), 1);
    assert_eq!(second.conflicts[0].cell_id, CellId(10));
    assert_eq!(
        second.conflicts[0].kind,
        AgentTransactionConflictKind::StaleCell
    );
    assert_eq!(
        db.get_latest_cell(CellId(10)).unwrap(),
        payload("shared:project", "agent 1")
    );
}

#[test]
fn concurrent_agent_transactions_allow_disjoint_cells_and_read_your_writes() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open_with_options(dir.path(), options()).unwrap();
    let base_seq = db.current_seq();

    let first = db
        .commit_agent_transaction(
            &view(1, "shared:project"),
            request(
                1,
                "shared:project",
                base_seq,
                WriteBatch::new().put_cell(CellId(11), payload("shared:project", "agent 1")),
            ),
        )
        .unwrap();
    let second = db
        .commit_agent_transaction(
            &view(2, "shared:project"),
            request(
                2,
                "shared:project",
                base_seq,
                WriteBatch::new().put_cell(CellId(12), payload("shared:project", "agent 2")),
            ),
        )
        .unwrap();

    assert_eq!(first.outcome, AgentTransactionOutcome::Committed);
    assert_eq!(second.outcome, AgentTransactionOutcome::Committed);
    // first: cell(11) at seq 1, then its idempotency-ledger entry at seq 2
    // (F04-B1.3 records a durable ledger cell after each committed txn); so the
    // second disjoint commit lands cell(12) at seq 3. The property under test is
    // that both disjoint transactions commit (no false conflict) + read-your-writes.
    assert_eq!(second.committed_seq, Some(CommitSeq(3)));
    assert_eq!(
        db.get_latest_cell(CellId(11)).unwrap(),
        payload("shared:project", "agent 1")
    );
    assert_eq!(
        db.get_latest_cell(CellId(12)).unwrap(),
        payload("shared:project", "agent 2")
    );
}

#[test]
fn agent_transaction_rejects_scope_mismatch_before_commit() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open_with_options(dir.path(), options()).unwrap();
    let error = db
        .commit_agent_transaction(
            &view(1, "shared:project"),
            request(
                1,
                "shared:project",
                db.current_seq(),
                WriteBatch::new().put_cell(CellId(20), payload("private:agent", "bad")),
            ),
        )
        .unwrap_err();

    assert!(matches!(error, EngineError::InvalidAgentSession(_)));
    assert!(db.get_latest_cell(CellId(20)).is_none());
}

#[test]
fn agent_transaction_rejects_unwritable_scope() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open_with_options(dir.path(), options()).unwrap();
    let error = db
        .commit_agent_transaction(
            &view(1, "other:scope"),
            request(
                1,
                "shared:project",
                db.current_seq(),
                WriteBatch::new().put_cell(CellId(21), payload("shared:project", "bad")),
            ),
        )
        .unwrap_err();

    assert_eq!(error.code().as_str(), "permission_denied");
    assert!(db.get_latest_cell(CellId(21)).is_none());
}
