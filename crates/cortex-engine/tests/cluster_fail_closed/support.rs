use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::thread;

use cortex_aql::{AgentId, AgentView, BrainId, MemoryType, RetrievalMode, Q16_ZERO};
use cortex_core::CellId;
use cortex_crypto::ReceiptSigningKey;
use cortex_engine::canonical::{canonical_context_pack_bytes, canonical_json_bytes};
use cortex_engine::{
    encode_snapshot_segment, scope_id, AppendEntriesRequest, ConsensusState,
    ContextAccessDecisionOutcome, ContextPackOptions, ContextPackReceiptEvidence, Database,
    DatabaseOptions, ElectionState, EngineFeatureFlags, InMemoryReplicationTransport, LogIndex,
    NodeId, ReplicationPeerServer, ReplicationPeerState, SnapshotChunk, SnapshotSegment,
    TcpReplicationTransport, Term,
};

const PUBLIC_SCOPE: &str = "project:investments";
const PRIVATE_SCOPE: &str = "project:private";
const RECEIPT_SEED: &str = "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f";
const DB_INSTANCE_ID: &str = "dbi_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const AUDIT_CHAIN_HEAD: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const RECEIPT_CREATED_UNIX_SECONDS: u64 = 1_700_000_000;

pub fn checked_evidence(db: &Database, expected_ids: &[CellId]) -> ContextPackReceiptEvidence {
    assert_binder_seed(db);
    let evidence = db
        .context_pack_with_receipt_evidence_from_aql(query(), &agent_view(), pack_options())
        .unwrap();
    assert_fail_closed_pack(&evidence, expected_ids);
    evidence
}

pub fn assert_same_pack_and_receipt(
    left: &ContextPackReceiptEvidence,
    right: &ContextPackReceiptEvidence,
) {
    assert_eq!(
        canonical_context_pack_bytes(&left.pack),
        canonical_context_pack_bytes(&right.pack)
    );
    assert_eq!(left.determinism_hash(), right.determinism_hash());
    let signing_key = ReceiptSigningKey::from_seed_hex("cluster-receipt-key", RECEIPT_SEED)
        .expect("fixture receipt key is valid");
    let left_receipt = left
        .signed_receipt_value(
            None,
            DB_INSTANCE_ID,
            RECEIPT_CREATED_UNIX_SECONDS,
            AUDIT_CHAIN_HEAD,
            &signing_key,
        )
        .unwrap();
    let right_receipt = right
        .signed_receipt_value(
            None,
            DB_INSTANCE_ID,
            RECEIPT_CREATED_UNIX_SECONDS,
            AUDIT_CHAIN_HEAD,
            &signing_key,
        )
        .unwrap();
    assert_eq!(
        canonical_json_bytes(&left_receipt),
        canonical_json_bytes(&right_receipt)
    );
}

pub fn install_snapshot_over_peer(follower_path: &Path, snapshot: SnapshotSegment) {
    let encoded = encode_snapshot_segment(&snapshot).unwrap();
    let split_at = encoded.len() / 2;
    let server = ReplicationPeerServer::bind(
        "127.0.0.1:0",
        ReplicationPeerState {
            election: follower_state(NodeId(2), &five_voters()),
            log: Vec::new(),
            snapshot: Vec::new(),
        },
        Some("secret".to_owned()),
    )
    .unwrap();
    let addr = server.local_addr().unwrap().to_string();
    let follower_path = follower_path.to_owned();
    let handle = thread::spawn(move || {
        let mut follower = open_replication_db(&follower_path);
        server
            .serve_n_with_snapshot_install(2, &mut follower)
            .unwrap();
        follower.close().unwrap();
    });
    let transport =
        TcpReplicationTransport::with_token(BTreeMap::from([(NodeId(2), addr)]), "secret".into());
    for (chunk_index, (payload, last)) in [
        (encoded[..split_at].to_vec(), false),
        (encoded[split_at..].to_vec(), true),
    ]
    .into_iter()
    .enumerate()
    {
        transport
            .send_snapshot_chunk(
                NodeId(2),
                &SnapshotChunk {
                    term: Term(2),
                    leader_id: NodeId(1),
                    leader_commit: LogIndex(snapshot.checkpoint_seq.0),
                    chunk_index: chunk_index as u64,
                    last,
                    payload,
                },
            )
            .unwrap();
    }
    handle.join().unwrap();
}

pub fn append_request(
    leader: &ConsensusState,
    entries: Vec<cortex_engine::ReplicatedEntry>,
    prev_log_index: LogIndex,
    prev_log_term: Term,
) -> AppendEntriesRequest {
    AppendEntriesRequest {
        term: leader.current_term,
        leader_id: leader.local_node,
        prev_log_index,
        prev_log_term,
        entries,
        leader_commit: leader.commit_index,
    }
}

pub fn transport_with_followers(
    voters: &BTreeSet<NodeId>,
    followers: &[NodeId],
) -> InMemoryReplicationTransport {
    let mut transport = InMemoryReplicationTransport::default();
    for follower in followers {
        transport.register_peer(follower_state(*follower, voters));
    }
    transport
}

pub fn seed_committed_cells(db: &mut Database) {
    db.put_cell(
        CellId(1),
        ready_project("source-a", "alpha budget approved"),
    )
    .unwrap();
    db.put_cell(
        CellId(2),
        ready_project("source-b", "alpha budget has signed evidence"),
    )
    .unwrap();
    db.put_cell(
        CellId(3),
        private_ready("private", "private answer must never be served"),
    )
    .unwrap();
}

pub fn seed_stale_follower(path: &Path) {
    let mut db = open_replication_db(path);
    db.put_cell(
        CellId(70),
        ready_project("stale", "stale old leader follower residue"),
    )
    .unwrap();
    db.put_cell(CellId(71), private_ready("stale", "private answer residue"))
        .unwrap();
    db.close().unwrap();
}

pub fn open_replication_db(path: &Path) -> Database {
    Database::open_with_options(
        path,
        DatabaseOptions {
            feature_flags: EngineFeatureFlags::production_safe()
                .with_experimental_replication(true),
            ..DatabaseOptions::default()
        },
    )
    .unwrap()
}

pub fn ready_project(source: &str, body: &str) -> Vec<u8> {
    format!("scope={PUBLIC_SCOPE}\nstatus=ready\nsource={source}\n\n{body}").into_bytes()
}

pub fn private_ready(source: &str, body: &str) -> Vec<u8> {
    format!("scope={PRIVATE_SCOPE}\nstatus=ready\nsource={source}\n\n{body}").into_bytes()
}

pub fn five_voters() -> BTreeSet<NodeId> {
    BTreeSet::from([NodeId(1), NodeId(2), NodeId(3), NodeId(4), NodeId(5)])
}

fn assert_binder_seed(db: &Database) {
    let report = db
        .explain_retrieve_aql(explain_query(), &agent_view())
        .unwrap();
    assert!(report.bitmap_ops.iter().any(|op| op == "PushAgentAllowed"));
    assert!(report.bitmap_ops.iter().any(|op| op == "PushLive"));
    assert!(report.bitmap_ops.iter().any(|op| op == "And"));
}

fn assert_fail_closed_pack(evidence: &ContextPackReceiptEvidence, expected_ids: &[CellId]) {
    let actual = evidence
        .pack
        .cells
        .iter()
        .map(|cell| cell.cell_id)
        .collect::<BTreeSet<_>>();
    assert_eq!(
        actual,
        expected_ids.iter().copied().collect::<BTreeSet<_>>()
    );
    for cell in &evidence.pack.cells {
        assert_eq!(cell.metadata.scope, PUBLIC_SCOPE);
        let body = String::from_utf8_lossy(&cell.payload);
        assert!(!body.contains(PRIVATE_SCOPE));
        assert!(!body.contains("private answer"));
        assert!(!body.contains("stale old leader"));
        let decision = cell.access_decision.as_ref().unwrap();
        assert_eq!(decision.decision, ContextAccessDecisionOutcome::Allowed);
        assert_eq!(decision.policy, "agent_view_readable_scope");
        assert_eq!(
            decision.policy_version.as_deref(),
            Some("agent_view_readable_scope.v1")
        );
    }
}

fn follower_state(local_node: NodeId, voters: &BTreeSet<NodeId>) -> ElectionState {
    let mut state = ElectionState::new(local_node, voters.clone());
    state.current_term = Term(1);
    state
}

fn query() -> &'static str {
    r#"RETRIEVE CONTEXT FOR TASK "alpha budget approval" IN BRAIN investment_projects
WHERE status = "ready" LIMIT 10 CANDIDATES;"#
}

fn explain_query() -> &'static str {
    r#"EXPLAIN RETRIEVE CONTEXT FOR TASK "alpha budget approval" IN BRAIN investment_projects
WHERE status = "ready" LIMIT 10 CANDIDATES;"#
}

fn pack_options() -> ContextPackOptions {
    ContextPackOptions {
        token_budget_tokens: 512,
        require_citations: false,
        ..ContextPackOptions::default()
    }
}

fn agent_view() -> AgentView {
    AgentView {
        agent_id: AgentId(1),
        label: Some("cluster-fail-closed".to_owned()),
        readable_brains: BTreeSet::from([BrainId(1)]),
        readable_scopes: BTreeSet::from([scope_id(PUBLIC_SCOPE)]),
        writable_scopes: BTreeSet::new(),
        allowed_modes: BTreeSet::from([RetrievalMode::Balanced]),
        allowed_memory_types: BTreeSet::from([MemoryType::Decision]),
        max_context_budget_tokens: 1_000,
        default_context_budget_tokens: 512,
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
