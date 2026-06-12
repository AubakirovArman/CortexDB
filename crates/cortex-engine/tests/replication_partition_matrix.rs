use std::collections::{BTreeMap, BTreeSet};
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::Path;
use std::thread;

use cortex_core::{CellId, CommitSeq};
use cortex_engine::{
    decode_snapshot_segment, encode_snapshot_segment, AppendEntriesRequest, ConsensusState,
    Database, DatabaseOptions, ElectionRole, ElectionState, EngineFeatureFlags,
    InMemoryReplicationTransport, LogIndex, NodeId, ReplicatedEntry, ReplicationLog,
    ReplicationPeerServer, ReplicationPeerState, ReplicationTransport, SnapshotChunk,
    SnapshotSegment, TcpReplicationTransport, Term,
};
use cortex_storage::segment::SegmentCell;

#[test]
fn five_node_partition_matrix_blocks_minority_and_commits_after_heal() {
    let voters = five_voters();
    let mut leader = ConsensusState::new(NodeId(1), voters.clone());
    let entry = leader.append_local(b"partitioned write".to_vec());

    let mut transport =
        transport_with_followers(&voters, &[NodeId(2), NodeId(3), NodeId(4), NodeId(5)]);
    transport.set_partitions(&[
        BTreeSet::from([NodeId(1), NodeId(2)]),
        BTreeSet::from([NodeId(3), NodeId(4), NodeId(5)]),
    ]);

    let minority_acks = transport
        .replicate_to_best_effort(
            [NodeId(2), NodeId(3), NodeId(4), NodeId(5)],
            append_request(&leader, vec![entry.clone()], LogIndex(0), Term(0)),
        )
        .unwrap();
    let minority = leader.record_acks(entry.index, minority_acks);
    assert!(!minority.committed);
    assert_eq!(leader.commit_index, LogIndex(0));

    transport.heal_partitions();
    let healed_acks = transport
        .replicate_to_best_effort(
            [NodeId(2), NodeId(3), NodeId(4), NodeId(5)],
            append_request(&leader, vec![entry.clone()], LogIndex(0), Term(0)),
        )
        .unwrap();
    let healed = leader.record_acks(entry.index, healed_acks);
    assert!(healed.committed);
    assert_eq!(leader.commit_index, entry.index);
}

#[test]
fn partitioned_leader_restart_keeps_uncommitted_entry_until_heal() {
    let voters = five_voters();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("replication.aclog");
    let mut leader = ConsensusState::new(NodeId(1), voters.clone());
    let entry = leader.append_local(b"restart after partition".to_vec());
    persist_entries(&path, std::slice::from_ref(&entry));

    let mut transport =
        transport_with_followers(&voters, &[NodeId(2), NodeId(3), NodeId(4), NodeId(5)]);
    transport.set_partitions(&[
        BTreeSet::from([NodeId(1), NodeId(2)]),
        BTreeSet::from([NodeId(3), NodeId(4), NodeId(5)]),
    ]);
    let minority_acks = transport
        .replicate_to_best_effort(
            [NodeId(2), NodeId(3), NodeId(4), NodeId(5)],
            append_request(&leader, vec![entry.clone()], LogIndex(0), Term(0)),
        )
        .unwrap();
    assert!(!leader.record_acks(entry.index, minority_acks).committed);

    let mut recovered =
        ReplicationLog::recover_consensus(&path, NodeId(1), voters.clone(), leader.commit_index)
            .unwrap();
    assert_eq!(recovered.commit_index, LogIndex(0));
    assert_eq!(recovered.entries(), std::slice::from_ref(&entry));

    transport.heal_partitions();
    let healed_acks = transport
        .replicate_to_best_effort(
            [NodeId(2), NodeId(3), NodeId(4), NodeId(5)],
            append_request(
                &recovered,
                recovered.entries().to_vec(),
                LogIndex(0),
                Term(0),
            ),
        )
        .unwrap();
    let healed = recovered.record_acks(entry.index, healed_acks);
    assert!(healed.committed);
    assert_eq!(recovered.commit_index, entry.index);
}

#[test]
fn committed_leader_restart_catches_up_healed_followers() {
    let voters = five_voters();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("replication.aclog");
    let mut leader = ConsensusState::new(NodeId(1), voters.clone());
    let entry = leader.append_local(b"committed before restart".to_vec());
    persist_entries(&path, std::slice::from_ref(&entry));

    let mut transport =
        transport_with_followers(&voters, &[NodeId(2), NodeId(3), NodeId(4), NodeId(5)]);
    transport.set_partitions(&[
        BTreeSet::from([NodeId(1), NodeId(2), NodeId(3)]),
        BTreeSet::from([NodeId(4), NodeId(5)]),
    ]);
    let majority_acks = transport
        .replicate_to_best_effort(
            [NodeId(2), NodeId(3), NodeId(4), NodeId(5)],
            append_request(&leader, vec![entry.clone()], LogIndex(0), Term(0)),
        )
        .unwrap();
    assert!(leader.record_acks(entry.index, majority_acks).committed);

    let recovered =
        ReplicationLog::recover_consensus(&path, NodeId(1), voters, leader.commit_index).unwrap();
    assert_eq!(recovered.commit_index, entry.index);

    transport.heal_partitions();
    let healed_request = append_request(
        &recovered,
        recovered.entries().to_vec(),
        LogIndex(0),
        Term(0),
    );
    let healed_acks = transport
        .replicate_to_best_effort([NodeId(2), NodeId(3), NodeId(4), NodeId(5)], healed_request)
        .unwrap();
    assert!(healed_acks.contains(&NodeId(4)));
    assert!(healed_acks.contains(&NodeId(5)));
    assert_eq!(transport.peer_commit(NodeId(4)), Some(entry.index));
    assert_eq!(transport.peer_commit(NodeId(5)), Some(entry.index));
}

#[test]
fn majority_partition_elects_new_leader_and_rejects_old_after_heal() {
    let voters = five_voters();
    let mut transport =
        transport_with_followers(&voters, &[NodeId(1), NodeId(2), NodeId(4), NodeId(5)]);
    transport.set_partitions(&[
        BTreeSet::from([NodeId(1), NodeId(2)]),
        BTreeSet::from([NodeId(3), NodeId(4), NodeId(5)]),
    ]);

    let mut candidate = ElectionState::new(NodeId(3), voters.clone());
    candidate.current_term = Term(1);
    let vote_request = candidate.start_election();
    assert!(transport
        .request_vote(NodeId(1), vote_request.clone())
        .is_err());

    let vote4 = transport
        .request_vote(NodeId(4), vote_request.clone())
        .unwrap();
    let vote5 = transport.request_vote(NodeId(5), vote_request).unwrap();
    assert!(!candidate.record_vote(vote4).elected);
    let outcome = candidate.record_vote(vote5);
    assert!(outcome.elected);
    assert_eq!(outcome.role, ElectionRole::Leader);

    transport.heal_partitions();
    let mut old_leader = ConsensusState::new(NodeId(1), voters);
    old_leader.current_term = Term(1);
    let stale_entry = old_leader.append_local(b"stale minority write".to_vec());
    let response = transport
        .append_entries(
            NodeId(4),
            AppendEntriesRequest {
                term: old_leader.current_term,
                leader_id: old_leader.local_node,
                prev_log_index: LogIndex(0),
                prev_log_term: Term(0),
                entries: vec![stale_entry],
                leader_commit: LogIndex(0),
            },
        )
        .unwrap();

    assert!(!response.success);
    assert!(transport.peer_log(NodeId(4)).unwrap().is_empty());
}

#[test]
fn real_peer_transport_streams_chunked_snapshot_payload() {
    let snapshot = SnapshotSegment {
        checkpoint_seq: CommitSeq(17),
        cells: vec![SegmentCell {
            candidate_id: 1,
            cell_id: 99,
            created_seq: 17,
            deleted_seq: None,
            payload: b"scope=project:investments\nstatus=ready\n\ntransport snapshot".to_vec(),
        }
        .into()],
    };
    let encoded = encode_snapshot_segment(&snapshot).unwrap();
    let split_at = encoded.len() / 2;

    let server = ReplicationPeerServer::bind(
        "127.0.0.1:0",
        ReplicationPeerState {
            election: follower_state(NodeId(2), &three_voters()),
            log: Vec::new(),
            snapshot: Vec::new(),
        },
        Some("secret".to_owned()),
    )
    .unwrap();
    let addr = server.local_addr().unwrap().to_string();
    let handle = thread::spawn(move || {
        server.serve_n(2).unwrap();
        server.state().unwrap()
    });

    let transport =
        TcpReplicationTransport::with_token(BTreeMap::from([(NodeId(2), addr)]), "secret".into());
    let first_len = transport
        .send_snapshot_chunk(
            NodeId(2),
            &SnapshotChunk {
                term: Term(2),
                leader_id: NodeId(1),
                leader_commit: LogIndex(17),
                chunk_index: 0,
                last: false,
                payload: encoded[..split_at].to_vec(),
            },
        )
        .unwrap();
    let full_len = transport
        .send_snapshot_chunk(
            NodeId(2),
            &SnapshotChunk {
                term: Term(2),
                leader_id: NodeId(1),
                leader_commit: LogIndex(17),
                chunk_index: 1,
                last: true,
                payload: encoded[split_at..].to_vec(),
            },
        )
        .unwrap();
    let state = handle.join().unwrap();

    assert_eq!(first_len as usize, split_at);
    assert_eq!(full_len as usize, encoded.len());
    assert_eq!(decode_snapshot_segment(&state.snapshot).unwrap(), snapshot);
}

#[test]
fn peer_snapshot_transport_installs_durable_follower_snapshot() {
    let dir = tempfile::tempdir().unwrap();
    {
        let mut follower = open_replication_database(dir.path());
        follower
            .put_cell(
                CellId(7),
                b"scope=old\nstatus=stale\n\nstale follower state".to_vec(),
            )
            .unwrap();
        follower.close().unwrap();
    }

    let snapshot = SnapshotSegment {
        checkpoint_seq: CommitSeq(23),
        cells: vec![SegmentCell {
            candidate_id: 1,
            cell_id: 99,
            created_seq: 23,
            deleted_seq: None,
            payload: b"scope=project:investments\nstatus=ready\n\ninstalled over peer".to_vec(),
        }
        .into()],
    };
    let encoded = encode_snapshot_segment(&snapshot).unwrap();
    let split_at = encoded.len() / 2;
    let server = ReplicationPeerServer::bind(
        "127.0.0.1:0",
        ReplicationPeerState {
            election: follower_state(NodeId(2), &three_voters()),
            log: Vec::new(),
            snapshot: Vec::new(),
        },
        Some("secret".to_owned()),
    )
    .unwrap();
    let addr = server.local_addr().unwrap().to_string();
    let follower_path = dir.path().to_owned();
    let handle = thread::spawn(move || {
        let mut follower = open_replication_database(&follower_path);
        server
            .serve_n_with_snapshot_install(2, &mut follower)
            .unwrap();
        follower.close().unwrap();
        server.state().unwrap()
    });
    let transport =
        TcpReplicationTransport::with_token(BTreeMap::from([(NodeId(2), addr)]), "secret".into());

    let first_len = transport
        .send_snapshot_chunk(
            NodeId(2),
            &SnapshotChunk {
                term: Term(3),
                leader_id: NodeId(1),
                leader_commit: LogIndex(23),
                chunk_index: 0,
                last: false,
                payload: encoded[..split_at].to_vec(),
            },
        )
        .unwrap();
    let full_len = transport
        .send_snapshot_chunk(
            NodeId(2),
            &SnapshotChunk {
                term: Term(3),
                leader_id: NodeId(1),
                leader_commit: LogIndex(23),
                chunk_index: 1,
                last: true,
                payload: encoded[split_at..].to_vec(),
            },
        )
        .unwrap();
    let state = handle.join().unwrap();
    let follower = Database::open(dir.path()).unwrap();

    assert_eq!(first_len as usize, split_at);
    assert_eq!(full_len as usize, encoded.len());
    assert_eq!(decode_snapshot_segment(&state.snapshot).unwrap(), snapshot);
    assert_eq!(
        follower.get_latest_cell(CellId(99)).unwrap(),
        snapshot.cells[0].payload
    );
    assert!(follower.get_latest_cell(CellId(7)).is_none());
    assert_eq!(follower.manifest().checkpoint_seq, 23);
}

#[test]
fn snapshot_frame_rejects_nonzero_first_chunk() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap().to_string();
    let handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut request = String::new();
        stream.read_to_string(&mut request).unwrap();
        let mut state = follower_state(NodeId(2), &three_voters());
        let mut log = Vec::new();
        let mut snapshot = Vec::new();
        let response = cortex_engine::handle_authenticated_replication_frame(
            &mut state,
            &mut log,
            &mut snapshot,
            None,
            &request,
        )
        .unwrap();
        stream.write_all(response.as_bytes()).unwrap();
        snapshot
    });

    let transport = TcpReplicationTransport::new(BTreeMap::from([(NodeId(2), addr)]));
    let result = transport.send_snapshot_chunk(
        NodeId(2),
        &SnapshotChunk {
            term: Term(1),
            leader_id: NodeId(1),
            leader_commit: LogIndex(1),
            chunk_index: 1,
            last: true,
            payload: b"missing first chunk".to_vec(),
        },
    );

    assert!(result.is_err());
    assert!(handle.join().unwrap().is_empty());
}

fn append_request(
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

fn persist_entries(path: &Path, entries: &[ReplicatedEntry]) {
    let log = ReplicationLog::open(path).unwrap();
    for entry in entries {
        log.append(entry).unwrap();
    }
    log.close().unwrap();
}

fn transport_with_followers(
    voters: &BTreeSet<NodeId>,
    followers: &[NodeId],
) -> InMemoryReplicationTransport {
    let mut transport = InMemoryReplicationTransport::default();
    for follower in followers {
        transport.register_peer(follower_state(*follower, voters));
    }
    transport
}

fn follower_state(local_node: NodeId, voters: &BTreeSet<NodeId>) -> ElectionState {
    let mut state = ElectionState::new(local_node, voters.clone());
    state.current_term = Term(1);
    state
}

fn three_voters() -> BTreeSet<NodeId> {
    BTreeSet::from([NodeId(1), NodeId(2), NodeId(3)])
}

fn five_voters() -> BTreeSet<NodeId> {
    BTreeSet::from([NodeId(1), NodeId(2), NodeId(3), NodeId(4), NodeId(5)])
}

fn open_replication_database(path: &Path) -> Database {
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
