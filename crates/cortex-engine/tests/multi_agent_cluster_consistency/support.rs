use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::thread;

use cortex_aql::{AgentId, AgentView, BrainId, MemoryType, RetrievalMode, Q16_ZERO};
use cortex_core::{CellId, CommitSeq};
use cortex_engine::{
    encode_snapshot_segment, scope_id, AgentHandoffRequest, AgentTransactionOptions,
    AgentTransactionRequest, ConsensusState, Database, DatabaseOptions, ElectionRole,
    ElectionState, EngineFeatureFlags, InMemoryReplicationTransport, LogIndex, NodeId,
    ReplicationPeerServer, ReplicationPeerState, ReplicationTransport, SnapshotChunk,
    SnapshotSegment, TcpReplicationTransport, Term, WriteBatch,
};

pub const SHARED_SCOPE: &str = "shared:project";

pub fn open_cluster_db(path: &Path) -> Database {
    Database::open_with_options(
        path,
        DatabaseOptions {
            agent_transactions: AgentTransactionOptions { enabled: true },
            feature_flags: EngineFeatureFlags::production_safe()
                .with_experimental_replication(true),
            ..DatabaseOptions::default()
        },
    )
    .unwrap()
}

pub fn commit_shared(
    db: &mut Database,
    view: &AgentView,
    cell_id: CellId,
    body: &str,
) -> CommitSeq {
    let base_seq = db.current_seq();
    db.commit_agent_transaction(
        view,
        AgentTransactionRequest {
            agent_id: view.agent_id,
            scope: SHARED_SCOPE.to_owned(),
            base_seq,
            batch: WriteBatch::new().put_cell(cell_id, payload(body)),
            idempotency_key: None,
        },
    )
    .unwrap()
    .committed_seq
    .unwrap()
}

pub fn assert_visible(db: &Database, view: &AgentView, expected: &[CellId]) {
    let actual = db
        .retrieve_aql(query(), view)
        .unwrap()
        .into_iter()
        .map(|cell| cell.cell_id)
        .collect::<BTreeSet<_>>();
    assert_eq!(actual, expected.iter().copied().collect::<BTreeSet<_>>());
}

pub fn handoff_request(
    first_seq: CommitSeq,
    pack_seq: CommitSeq,
    required_after_seq: CommitSeq,
    suffix: &str,
) -> AgentHandoffRequest {
    AgentHandoffRequest {
        source_agent_id: AgentId(1),
        target_agent_id: AgentId(2),
        scope: SHARED_SCOPE.to_owned(),
        pack_hash: format!("ctxpack:v1:{suffix}:{}", first_seq.0),
        pack_seq,
        required_after_seq,
        idempotency_key: Some(format!("cluster-handoff-{suffix}")),
    }
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
        let mut follower = open_cluster_db(&follower_path);
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

pub fn elect_new_leader_after_failover() {
    let voters = five_voters();
    let mut transport = transport_with_followers(&voters, &[NodeId(1), NodeId(3)]);
    let mut candidate = ElectionState::new(NodeId(2), voters);
    candidate.current_term = Term(1);
    let request = candidate.start_election();
    assert!(
        !candidate
            .record_vote(transport.request_vote(NodeId(1), request.clone()).unwrap())
            .elected
    );
    let outcome = candidate.record_vote(transport.request_vote(NodeId(3), request).unwrap());
    assert!(outcome.elected);
    assert_eq!(outcome.role, ElectionRole::Leader);
}

pub fn append_request(
    leader: &ConsensusState,
    entries: Vec<cortex_engine::ReplicatedEntry>,
    prev_log_index: LogIndex,
    prev_log_term: Term,
) -> cortex_engine::AppendEntriesRequest {
    cortex_engine::AppendEntriesRequest {
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

pub fn view(agent_id: u64, readable: &[&str], writable: &[&str]) -> AgentView {
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
        private_scope: None,
    }
}

pub fn five_voters() -> BTreeSet<NodeId> {
    BTreeSet::from([NodeId(1), NodeId(2), NodeId(3), NodeId(4), NodeId(5)])
}

fn follower_state(local_node: NodeId, voters: &BTreeSet<NodeId>) -> ElectionState {
    let mut state = ElectionState::new(local_node, voters.clone());
    state.current_term = Term(1);
    state
}

fn payload(body: &str) -> Vec<u8> {
    format!("scope={SHARED_SCOPE}\nstatus=ready\ntype=fact\n\n{body}").into_bytes()
}

fn query() -> &'static str {
    r#"RETRIEVE CONTEXT FOR TASK "cluster" IN BRAIN investment_projects
WHERE status = "ready" LIMIT 10 CANDIDATES;"#
}
