use std::collections::BTreeSet;

use cortex_aql::{AgentId, AgentView, BrainId, MemoryType, RetrievalMode, Q16_ZERO};
use cortex_core::{CellId, CommitSeq};
use cortex_engine::{
    classify_memory_visibility, scope_id, AgentHandoffRequest, AgentTransactionConflictKind,
    AgentTransactionOptions, AgentTransactionOutcome, AgentTransactionRequest, Database,
    DatabaseOptions, EngineError, MemoryConsistencyLevel, WriteBatch,
};
use cortex_storage::wal::{WalReader, WalRecordType};

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
                receipt_pack_root: None,
                receipt_signature_context: None,
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
                receipt_pack_root: None,
                receipt_signature_context: None,
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
                receipt_pack_root: None,
                receipt_signature_context: None,
            },
        )
        .unwrap_err();
    assert!(matches!(future_seq, EngineError::InvalidAgentSession(_)));
}

#[test]
fn require_seq_visible_enforces_read_after_seq() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open_with_options(dir.path(), options()).unwrap();

    let base = db.current_seq();
    // The current sequence is always visible.
    assert_eq!(db.require_seq_visible(base).unwrap(), base);

    // A future sequence is not yet visible: a hard, typed failure — never a
    // silently-stale read.
    let error = db.require_seq_visible(CommitSeq(base.0 + 1)).unwrap_err();
    match error {
        EngineError::SequenceNotVisible { required, current } => {
            assert_eq!(required, CommitSeq(base.0 + 1));
            assert_eq!(current, base);
        }
        other => panic!("expected SequenceNotVisible, got {other:?}"),
    }

    // Once a commit advances the sequence, the once-future seq becomes visible.
    db.put_cell(CellId(7), payload("shared:project", "advance"))
        .unwrap();
    let advanced = db.current_seq();
    assert!(advanced > base);
    assert_eq!(
        db.require_seq_visible(CommitSeq(base.0 + 1)).unwrap(),
        advanced
    );
}

fn handoff_request(pack_seq: CommitSeq) -> AgentHandoffRequest {
    AgentHandoffRequest {
        source_agent_id: AgentId(1),
        target_agent_id: AgentId(2),
        scope: "shared:project".to_owned(),
        pack_hash: "ctxpack:v1:gamma".to_owned(),
        pack_seq,
        required_after_seq: CommitSeq(0),
        idempotency_key: Some("handoff-1-2".to_owned()),
        receipt_pack_root: None,
        receipt_signature_context: None,
    }
}

#[test]
fn commit_agent_handoff_requires_feature_flag() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    let source = view(1, &["shared:project"], &["shared:project"], None);
    let target = view(2, &["shared:project"], &[], None);

    let error = db
        .commit_agent_handoff(&source, &target, handoff_request(db.current_seq()))
        .unwrap_err();
    assert!(matches!(
        error,
        EngineError::FeatureDisabled("agent_transactions")
    ));
}

#[test]
fn commit_agent_handoff_persists_reads_back_and_never_leaks() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open_with_options(dir.path(), options()).unwrap();
    let source = view(1, &["shared:project"], &["shared:project"], None);
    let target = view(2, &["shared:project"], &[], None);

    // A normal shared cell that retrieval *should* return.
    db.commit_agent_transaction(
        &source,
        request(
            1,
            "shared:project",
            db.current_seq(),
            WriteBatch::new().put_cell(CellId(2), payload("shared:project", "shared beta")),
        ),
    )
    .unwrap();

    let committed = db
        .commit_agent_handoff(&source, &target, handoff_request(db.current_seq()))
        .unwrap();
    assert_eq!(
        committed.report.level,
        MemoryConsistencyLevel::SharedSequenced
    );
    assert_eq!(
        committed.handoff_cell_id.0 & 0xf000_0000_0000_0000,
        0xc000_0000_0000_0000,
        "handoff record lands in the reserved 0xc namespace"
    );

    // Read the durable record back for audit.
    let read_back = db
        .read_agent_handoff(committed.handoff_cell_id)
        .unwrap()
        .unwrap();
    assert_eq!(read_back, committed.report);

    // Retrieval returns the normal cell but never the reserved handoff record.
    let ids = retrieve_ids(&db, &source, beta_query());
    assert_eq!(ids, vec![CellId(2)]);
    assert!(ids
        .iter()
        .all(|cell| cell.0 & 0xf000_0000_0000_0000 != 0xc000_0000_0000_0000));
}

#[test]
fn committed_agent_handoff_survives_restart() {
    let dir = tempfile::tempdir().unwrap();
    let source = view(1, &["shared:project"], &["shared:project"], None);
    let target = view(2, &["shared:project"], &[], None);

    let (cell_id, report) = {
        let mut db = Database::open_with_options(dir.path(), options()).unwrap();
        let committed = db
            .commit_agent_handoff(&source, &target, handoff_request(db.current_seq()))
            .unwrap();
        (committed.handoff_cell_id, committed.report)
    };

    // Reopen: the persisted handoff record is still auditable.
    let db = Database::open_with_options(dir.path(), options()).unwrap();
    assert_eq!(db.read_agent_handoff(cell_id).unwrap().unwrap(), report);
}

#[test]
fn agent_handoff_records_receipt_binding() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open_with_options(dir.path(), options()).unwrap();
    let source = view(1, &["shared:project"], &["shared:project"], None);
    let target = view(2, &["shared:project"], &[], None);

    // C3-3: bind the handoff to the accountability receipt it carries.
    let mut request = handoff_request(db.current_seq());
    request.receipt_pack_root = Some("packroot:deadbeef".to_owned());
    request.receipt_signature_context = Some("sigctx:v1".to_owned());

    let committed = db.commit_agent_handoff(&source, &target, request).unwrap();
    assert_eq!(
        committed.report.receipt_pack_root.as_deref(),
        Some("packroot:deadbeef")
    );

    // The binding round-trips through the durable B6.1 ledger.
    let read_back = db
        .read_agent_handoff(committed.handoff_cell_id)
        .unwrap()
        .unwrap();
    assert_eq!(
        read_back.receipt_pack_root.as_deref(),
        Some("packroot:deadbeef")
    );
    assert_eq!(
        read_back.receipt_signature_context.as_deref(),
        Some("sigctx:v1")
    );
    // Equality with the committed report confirms full round-trip fidelity.
    assert_eq!(read_back, committed.report);
}

fn keyed_request(
    agent_id: u64,
    scope: &str,
    base_seq: CommitSeq,
    batch: WriteBatch,
    key: &str,
) -> AgentTransactionRequest {
    let mut request = request(agent_id, scope, base_seq, batch);
    request.idempotency_key = Some(key.to_owned());
    request
}

#[test]
fn idempotent_transaction_replays_without_rewriting() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open_with_options(dir.path(), options()).unwrap();
    let owner = view(1, &["agent:one"], &["agent:one"], Some("agent:one"));

    let req = keyed_request(
        1,
        "agent:one",
        db.current_seq(),
        WriteBatch::new().put_cell(CellId(1), payload("agent:one", "first")),
        "key-1",
    );
    let first = db.commit_agent_transaction(&owner, req.clone()).unwrap();
    assert_eq!(first.outcome, AgentTransactionOutcome::Committed);
    assert!(!first.idempotent_replay);
    let committed = first.committed_seq.unwrap();
    let seq_after_first = db.current_seq();

    // Replaying the identical request returns the SAME outcome and does not write.
    let second = db.commit_agent_transaction(&owner, req).unwrap();
    assert!(second.idempotent_replay);
    assert_eq!(second.committed_seq, Some(committed));
    assert_eq!(
        db.current_seq(),
        seq_after_first,
        "an idempotent replay must not advance the commit sequence"
    );
}

#[test]
fn reused_idempotency_key_with_different_request_is_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open_with_options(dir.path(), options()).unwrap();
    let owner = view(1, &["agent:one"], &["agent:one"], Some("agent:one"));

    let first = keyed_request(
        1,
        "agent:one",
        db.current_seq(),
        WriteBatch::new().put_cell(CellId(1), payload("agent:one", "first")),
        "dup",
    );
    db.commit_agent_transaction(&owner, first).unwrap();

    // Same key, different write => reuse is rejected (no silent replay of the wrong
    // result, no new write).
    let clashing = keyed_request(
        1,
        "agent:one",
        db.current_seq(),
        WriteBatch::new().put_cell(CellId(2), payload("agent:one", "second")),
        "dup",
    );
    let error = db.commit_agent_transaction(&owner, clashing).unwrap_err();
    assert!(matches!(error, EngineError::InvalidAgentSession(_)));
    assert!(
        db.get_latest_cell(CellId(2)).is_none(),
        "rejected write must not land"
    );
}

#[test]
fn distinct_idempotency_keys_execute_independently() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open_with_options(dir.path(), options()).unwrap();
    let owner = view(1, &["agent:one"], &["agent:one"], Some("agent:one"));

    let one = db
        .commit_agent_transaction(
            &owner,
            keyed_request(
                1,
                "agent:one",
                db.current_seq(),
                WriteBatch::new().put_cell(CellId(1), payload("agent:one", "a")),
                "k1",
            ),
        )
        .unwrap();
    let two = db
        .commit_agent_transaction(
            &owner,
            keyed_request(
                1,
                "agent:one",
                db.current_seq(),
                WriteBatch::new().put_cell(CellId(2), payload("agent:one", "b")),
                "k2",
            ),
        )
        .unwrap();
    assert!(!one.idempotent_replay && !two.idempotent_replay);
    assert!(db.get_latest_cell(CellId(1)).is_some());
    assert!(db.get_latest_cell(CellId(2)).is_some());
}

#[test]
fn idempotency_ledger_entries_never_leak_into_retrieval() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open_with_options(dir.path(), options()).unwrap();
    let owner = view(1, &["agent:one"], &["agent:one"], Some("agent:one"));

    db.commit_agent_transaction(
        &owner,
        keyed_request(
            1,
            "agent:one",
            db.current_seq(),
            WriteBatch::new().put_cell(CellId(1), payload("agent:one", "alpha rollout")),
            "leak-check",
        ),
    )
    .unwrap();

    // The agent sees only its own cell; the reserved-scope ledger entry (namespace
    // 0xb) is never a retrieval candidate.
    let ids = retrieve_ids(&db, &owner, alpha_query());
    assert_eq!(ids, vec![CellId(1)]);
    assert!(
        ids.iter()
            .all(|cell| cell.0 & 0xf000_0000_0000_0000 != 0xb000_0000_0000_0000),
        "no ledger-namespace cell may surface in retrieval"
    );
}

#[test]
fn idempotency_ledger_survives_restart() {
    let dir = tempfile::tempdir().unwrap();
    let owner = view(1, &["agent:one"], &["agent:one"], Some("agent:one"));
    let req = keyed_request(
        1,
        "agent:one",
        CommitSeq(0),
        WriteBatch::new().put_cell(CellId(1), payload("agent:one", "durable")),
        "restart-key",
    );

    let committed = {
        let mut db = Database::open_with_options(dir.path(), options()).unwrap();
        let report = db.commit_agent_transaction(&owner, req.clone()).unwrap();
        assert!(!report.idempotent_replay);
        report.committed_seq.unwrap()
    };

    // Reopen the same directory: the persisted ledger still replays the key.
    let mut reopened = Database::open_with_options(dir.path(), options()).unwrap();
    let replay = reopened.commit_agent_transaction(&owner, req).unwrap();
    assert!(
        replay.idempotent_replay,
        "ledger must persist across restart"
    );
    assert_eq!(replay.committed_seq, Some(committed));
}

#[test]
fn idempotency_ledger_and_mutation_share_one_wal_batch() {
    let dir = tempfile::tempdir().unwrap();
    let owner = view(1, &["agent:one"], &["agent:one"], Some("agent:one"));
    let mut db = Database::open_with_options(dir.path(), options()).unwrap();
    let report = db
        .commit_agent_transaction(
            &owner,
            keyed_request(
                1,
                "agent:one",
                CommitSeq(0),
                WriteBatch::new().put_cell(CellId(1), payload("agent:one", "atomic")),
                "atomic-key",
            ),
        )
        .unwrap();
    assert_eq!(report.committed_seq, Some(CommitSeq(1)));
    db.close().unwrap();

    let scan = WalReader::scan_path(dir.path().join("db.aclog")).unwrap();
    let record_types = scan
        .records
        .iter()
        .map(|record| record.record.record_type)
        .collect::<Vec<_>>();
    assert_eq!(
        record_types,
        vec![
            WalRecordType::WriteBatchBegin,
            WalRecordType::PutCellBatch,
            WalRecordType::PutCellBatch,
            WalRecordType::WriteBatchCommit,
        ],
        "the business mutation and ledger entry must be enclosed by one commit marker"
    );
}

fn alpha_query() -> &'static str {
    r#"RETRIEVE CONTEXT FOR TASK "alpha" IN BRAIN investment_projects
WHERE status = "ready" LIMIT 10 CANDIDATES;"#
}

fn beta_query() -> &'static str {
    r#"RETRIEVE CONTEXT FOR TASK "beta" IN BRAIN investment_projects
WHERE status = "ready" LIMIT 10 CANDIDATES;"#
}
