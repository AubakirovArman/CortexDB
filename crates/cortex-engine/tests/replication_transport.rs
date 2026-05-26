use std::collections::BTreeSet;

use cortex_core::{CellId, CommitSeq};
use cortex_engine::{
    decode_snapshot_segment, encode_snapshot_segment, plan_replication_recovery,
    AppendEntriesRequest, ConsensusState, Database, ElectionRole, ElectionState,
    InMemoryReplicationTransport, LogIndex, NodeId, ReplicationPeerServer, ReplicationPeerState,
    ReplicationRecoveryAction, ReplicationRecoveryPolicy, ReplicationTransport, SnapshotChunk,
    SnapshotSegment, TcpReplicationTransport, Term,
};
use cortex_storage::segment::SegmentCell;
use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;

#[test]
fn election_state_wins_majority_vote() {
    let voters = voters();
    let mut candidate = ElectionState::new(NodeId(1), voters.clone());
    let mut transport = transport_with_followers(&voters, &[NodeId(2), NodeId(3)]);
    let request = candidate.start_election();

    let first = transport.request_vote(NodeId(2), request.clone()).unwrap();
    let second = transport.request_vote(NodeId(3), request).unwrap();

    let outcome = candidate.record_vote(first);
    assert!(outcome.elected);
    assert_eq!(outcome.role, ElectionRole::Leader);
    assert_eq!(outcome.leader, Some(NodeId(1)));
    assert_eq!(candidate.record_vote(second).role, ElectionRole::Leader);
}

#[test]
fn stale_candidate_log_is_rejected() {
    let voters = voters();
    let mut follower = ElectionState::new(NodeId(2), voters.clone());
    follower.set_last_log(LogIndex(3), Term(2));
    let mut candidate = ElectionState::new(NodeId(1), voters);
    candidate.set_last_log(LogIndex(2), Term(1));
    let request = candidate.start_election();

    let response = follower.handle_vote_request(&request);

    assert!(!response.vote_granted);
    assert_eq!(response.term, request.term);
}

#[test]
fn append_entries_rejects_stale_term_and_accepts_current_leader() {
    let voters = voters();
    let mut transport = transport_with_followers(&voters, &[NodeId(2)]);
    let mut leader = ConsensusState::new(NodeId(1), voters.clone());
    let entry = leader.append_local(b"put cell".to_vec());

    let stale = transport
        .append_entries(
            NodeId(2),
            AppendEntriesRequest {
                term: Term(0),
                leader_id: NodeId(1),
                prev_log_index: LogIndex(0),
                prev_log_term: Term(0),
                entries: vec![entry.clone()],
                leader_commit: LogIndex(0),
            },
        )
        .unwrap();
    assert!(!stale.success);

    let accepted = transport
        .append_entries(
            NodeId(2),
            AppendEntriesRequest {
                term: Term(1),
                leader_id: NodeId(1),
                prev_log_index: LogIndex(0),
                prev_log_term: Term(0),
                entries: vec![entry.clone()],
                leader_commit: LogIndex(1),
            },
        )
        .unwrap();
    assert!(accepted.success);
    assert_eq!(accepted.match_index, entry.index);
    assert_eq!(transport.peer_log(NodeId(2)).unwrap(), &[entry]);
}

#[test]
fn replicated_entry_acks_can_commit_consensus_state() {
    let voters = voters();
    let mut leader = ConsensusState::new(NodeId(1), voters.clone());
    let entry = leader.append_local(b"put cell".to_vec());
    let mut transport = transport_with_followers(&voters, &[NodeId(2), NodeId(3)]);
    let acks = transport
        .replicate_to(
            [NodeId(2), NodeId(3)],
            AppendEntriesRequest {
                term: leader.current_term,
                leader_id: NodeId(1),
                prev_log_index: LogIndex(0),
                prev_log_term: Term(0),
                entries: vec![entry.clone()],
                leader_commit: LogIndex(0),
            },
        )
        .unwrap();

    let decision = leader.record_acks(entry.index, acks);

    assert!(decision.committed);
    assert_eq!(leader.committed_entries(), vec![entry]);
}

#[test]
fn tcp_replication_transport_sends_append_entries() {
    let follower_voters = voters();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap().to_string();
    let handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut request = String::new();
        stream.read_to_string(&mut request).unwrap();
        let mut state = ElectionState::new(NodeId(2), follower_voters);
        state.current_term = Term(1);
        let mut log = Vec::new();
        let response =
            cortex_engine::handle_replication_frame(&mut state, &mut log, &request).unwrap();
        stream.write_all(response.as_bytes()).unwrap();
        log
    });

    let mut leader = ConsensusState::new(NodeId(1), voters());
    let entry = leader.append_local(b"put cell".to_vec());
    let mut transport = TcpReplicationTransport::new(BTreeMap::from([(NodeId(2), addr)]));
    let response = transport
        .append_entries(
            NodeId(2),
            AppendEntriesRequest {
                term: Term(1),
                leader_id: NodeId(1),
                prev_log_index: LogIndex(0),
                prev_log_term: Term(0),
                entries: vec![entry.clone()],
                leader_commit: LogIndex(0),
            },
        )
        .unwrap();

    assert!(response.success);
    assert_eq!(response.match_index, LogIndex(1));
    assert_eq!(handle.join().unwrap(), vec![entry]);
}

#[test]
fn replication_peer_server_accepts_authenticated_snapshot_chunk() {
    let server = ReplicationPeerServer::bind(
        "127.0.0.1:0",
        ReplicationPeerState {
            election: follower_state(),
            log: Vec::new(),
            snapshot: Vec::new(),
        },
        Some("secret".to_owned()),
    )
    .unwrap();
    let addr = server.local_addr().unwrap().to_string();
    let handle = thread::spawn(move || server.serve_n(1).unwrap());
    let transport =
        TcpReplicationTransport::with_token(BTreeMap::from([(NodeId(2), addr)]), "secret".into());

    let received = transport
        .send_snapshot_chunk(
            NodeId(2),
            &SnapshotChunk {
                term: Term(1),
                leader_id: NodeId(1),
                leader_commit: LogIndex(9),
                chunk_index: 0,
                last: true,
                payload: b"snapshot".to_vec(),
            },
        )
        .unwrap();

    assert_eq!(received, 8);
    handle.join().unwrap();
}

#[test]
fn authenticated_replication_frame_rejects_wrong_token() {
    let mut state = follower_state();
    let mut log = Vec::new();
    let mut snapshot = Vec::new();

    let result = cortex_engine::handle_authenticated_replication_frame(
        &mut state,
        &mut log,
        &mut snapshot,
        Some("secret"),
        "AUTH wrong VOTE 1 1 0 0",
    );

    assert!(result.is_err());
}

#[test]
fn snapshot_segment_roundtrips_and_installs_durably() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    let snapshot = SnapshotSegment {
        checkpoint_seq: CommitSeq(7),
        cells: vec![SegmentCell {
            candidate_id: 1,
            cell_id: 99,
            created_seq: 7,
            deleted_seq: None,
            payload: b"scope=project:investments\nstatus=ready\n\nsnapshot cell".to_vec(),
        }],
    };
    let encoded = encode_snapshot_segment(&snapshot).unwrap();
    let decoded = decode_snapshot_segment(&encoded).unwrap();

    let stats = db.install_snapshot_segment(decoded).unwrap();
    db.close().unwrap();
    let db = Database::open(dir.path()).unwrap();

    assert_eq!(stats.checkpoint_seq, CommitSeq(7));
    assert_eq!(
        db.get_latest_cell(CellId(99)).unwrap(),
        snapshot.cells[0].payload
    );
    assert_eq!(db.manifest().live_segments.len(), 1);
}

#[test]
fn replication_recovery_planner_selects_append_or_snapshot() {
    let append = plan_replication_recovery(
        LogIndex(10),
        LogIndex(12),
        ReplicationRecoveryPolicy {
            snapshot_threshold: 10,
        },
    );
    assert_eq!(
        append.action,
        ReplicationRecoveryAction::AppendEntries {
            from_exclusive: LogIndex(10),
            to_inclusive: LogIndex(12),
        }
    );

    let snapshot = plan_replication_recovery(
        LogIndex(1),
        LogIndex(20),
        ReplicationRecoveryPolicy {
            snapshot_threshold: 10,
        },
    );
    assert_eq!(
        snapshot.action,
        ReplicationRecoveryAction::InstallSnapshot {
            checkpoint: LogIndex(20),
        }
    );
}

fn voters() -> BTreeSet<NodeId> {
    BTreeSet::from([NodeId(1), NodeId(2), NodeId(3)])
}

fn follower_state() -> ElectionState {
    let mut state = ElectionState::new(NodeId(2), voters());
    state.current_term = Term(1);
    state
}

fn transport_with_followers(
    voters: &BTreeSet<NodeId>,
    followers: &[NodeId],
) -> InMemoryReplicationTransport {
    let mut transport = InMemoryReplicationTransport::default();
    for follower in followers {
        let mut state = ElectionState::new(*follower, voters.clone());
        state.current_term = Term(1);
        transport.register_peer(state);
    }
    transport
}
