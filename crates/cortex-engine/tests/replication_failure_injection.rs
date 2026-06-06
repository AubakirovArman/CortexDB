use std::collections::{BTreeMap, BTreeSet};

use cortex_core::{CellId, CommitSeq};
use cortex_engine::{
    assemble_snapshot_chunks, decode_snapshot_segment, encode_snapshot_segment,
    AppendEntriesRequest, ConsensusState, Database, DatabaseOptions, ElectionRole, ElectionState,
    EngineFeatureFlags, InMemoryReplicationTransport, LogIndex, NodeId, ReplicationLog,
    ReplicationTransport, SnapshotChunk, SnapshotSegment, Term,
};
use cortex_storage::segment::SegmentCell;

#[test]
fn minority_partition_cannot_commit_until_majority_heals() {
    let voters = voters();
    let mut leader = ConsensusState::new(NodeId(1), voters.clone());
    let entry = leader.append_local(b"minority write".to_vec());

    let minority = leader.record_acks(entry.index, BTreeSet::from([NodeId(1)]));
    assert!(!minority.committed);
    assert_eq!(leader.commit_index, LogIndex(0));
    assert!(leader.committed_entries().is_empty());

    let healed = leader.record_acks(entry.index, BTreeSet::from([NodeId(1), NodeId(2)]));
    assert!(healed.committed);
    assert_eq!(leader.commit_index, entry.index);
    assert_eq!(leader.committed_entries(), vec![entry]);
}

#[test]
fn healed_majority_rejects_stale_partitioned_leader() {
    let voters = voters();
    let mut node3 = ElectionState::new(NodeId(3), voters.clone());
    node3.current_term = Term(1);

    let mut node2 = ElectionState::new(NodeId(2), voters.clone());
    node2.current_term = Term(1);
    let vote_request = node2.start_election();
    let vote_response = node3.handle_vote_request(&vote_request);
    assert!(vote_response.vote_granted);

    let outcome = node2.record_vote(vote_response);
    assert!(outcome.elected);
    assert_eq!(outcome.role, ElectionRole::Leader);
    assert_eq!(outcome.leader, Some(NodeId(2)));

    let mut old_leader = ConsensusState::new(NodeId(1), voters);
    old_leader.current_term = Term(1);
    let stale_entry = old_leader.append_local(b"stale write".to_vec());

    let rejected = node3.accept_leader(old_leader.current_term, NodeId(1));
    assert!(!rejected);
    assert_eq!(node3.current_term, Term(2));

    let mut transport = InMemoryReplicationTransport::default();
    transport.register_peer(node3);
    let accepted = transport
        .append_entries(
            NodeId(3),
            AppendEntriesRequest {
                term: old_leader.current_term,
                leader_id: NodeId(1),
                prev_log_index: LogIndex(0),
                prev_log_term: Term(0),
                entries: vec![stale_entry],
                leader_commit: LogIndex(0),
            },
        )
        .unwrap();
    assert!(!accepted.success);
    assert!(
        transport.peer_log(NodeId(3)).unwrap().is_empty(),
        "stale leader must not mutate follower log"
    );
}

#[test]
fn replication_log_replay_is_idempotent_after_restart() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("replication.aclog");
    let voters = voters();

    let mut leader = ConsensusState::new(NodeId(1), voters.clone());
    let first = leader.append_local(b"first".to_vec());
    let second = leader.append_local(b"second".to_vec());
    let decision = leader.record_match_indexes(BTreeMap::from([
        (NodeId(2), second.index),
        (NodeId(3), second.index),
    ]));
    assert!(decision.committed);

    {
        let log = ReplicationLog::open(&path).unwrap();
        log.append(&first).unwrap();
        log.append(&second).unwrap();
        log.close().unwrap();
    }

    let recovered_once =
        ReplicationLog::recover_consensus(&path, NodeId(1), voters.clone(), leader.commit_index)
            .unwrap();
    let recovered_twice =
        ReplicationLog::recover_consensus(&path, NodeId(1), voters.clone(), leader.commit_index)
            .unwrap();

    assert_eq!(recovered_once.entries(), recovered_twice.entries());
    assert_eq!(
        recovered_once.committed_entries(),
        vec![first.clone(), second.clone()]
    );
    assert_eq!(recovered_once.last_log_index(), LogIndex(2));

    let mut resumed = recovered_once;
    let third = resumed.append_local(b"third".to_vec());
    assert_eq!(third.index, LogIndex(3));
}

#[test]
fn chunked_snapshot_resync_installs_follower_and_survives_restart() {
    let dir = tempfile::tempdir().unwrap();
    let mut follower = open_replication_db(dir.path());
    follower
        .put_cell(CellId(7), b"stale local state".to_vec())
        .unwrap();

    let snapshot = SnapshotSegment {
        checkpoint_seq: CommitSeq(11),
        cells: vec![SegmentCell {
            candidate_id: 1,
            cell_id: 42,
            created_seq: 11,
            deleted_seq: None,
            payload: b"scope=project:investments\nstatus=ready\n\nresynced cell".to_vec(),
        }],
    };
    let encoded = encode_snapshot_segment(&snapshot).unwrap();
    let split_at = encoded.len() / 2;
    let chunks = vec![
        SnapshotChunk {
            term: Term(3),
            leader_id: NodeId(1),
            leader_commit: LogIndex(11),
            chunk_index: 0,
            last: false,
            payload: encoded[..split_at].to_vec(),
        },
        SnapshotChunk {
            term: Term(3),
            leader_id: NodeId(1),
            leader_commit: LogIndex(11),
            chunk_index: 1,
            last: true,
            payload: encoded[split_at..].to_vec(),
        },
    ];

    let assembled = assemble_snapshot_chunks(&chunks).unwrap();
    let decoded = decode_snapshot_segment(&assembled).unwrap();
    let stats = follower.install_snapshot_segment(decoded).unwrap();
    follower.close().unwrap();

    let follower = Database::open(dir.path()).unwrap();
    assert_eq!(stats.checkpoint_seq, CommitSeq(11));
    assert_eq!(
        follower.get_latest_cell(CellId(42)).unwrap(),
        snapshot.cells[0].payload
    );
    assert!(
        follower.get_latest_cell(CellId(7)).is_none(),
        "snapshot install should replace stale follower state"
    );
}

fn open_replication_db(path: &std::path::Path) -> Database {
    Database::open_with_options(
        path,
        DatabaseOptions {
            feature_flags: EngineFeatureFlags {
                experimental_replication: true,
                ..EngineFeatureFlags::production_safe()
            },
            ..DatabaseOptions::default()
        },
    )
    .unwrap()
}

#[test]
fn snapshot_reassembly_rejects_missing_or_mixed_chunks() {
    let first = SnapshotChunk {
        term: Term(3),
        leader_id: NodeId(1),
        leader_commit: LogIndex(11),
        chunk_index: 0,
        last: false,
        payload: b"first".to_vec(),
    };
    let skipped = SnapshotChunk {
        chunk_index: 2,
        last: true,
        payload: b"third".to_vec(),
        ..first.clone()
    };
    let mixed_leader = SnapshotChunk {
        leader_id: NodeId(2),
        chunk_index: 1,
        last: true,
        payload: b"second".to_vec(),
        ..first.clone()
    };

    assert!(assemble_snapshot_chunks(&[first.clone(), skipped]).is_err());
    assert!(assemble_snapshot_chunks(&[first, mixed_leader]).is_err());
}

#[test]
fn membership_reconfiguration_changes_majority_counting() {
    let mut leader = ConsensusState::new(NodeId(1), voters());
    let first = leader.append_local(b"first".to_vec());

    let non_voter_only = leader.record_acks(first.index, BTreeSet::from([NodeId(1), NodeId(4)]));
    assert!(!non_voter_only.committed);

    leader
        .reconfigure_voters(BTreeSet::from([
            NodeId(1),
            NodeId(2),
            NodeId(3),
            NodeId(4),
            NodeId(5),
        ]))
        .unwrap();
    let joined_majority = leader.record_acks(
        first.index,
        BTreeSet::from([NodeId(1), NodeId(4), NodeId(5)]),
    );
    assert!(joined_majority.committed);

    let second = leader.append_local(b"second".to_vec());
    leader.reconfigure_voters(voters()).unwrap();
    let removed_nodes = leader.record_acks(
        second.index,
        BTreeSet::from([NodeId(1), NodeId(4), NodeId(5)]),
    );
    assert!(!removed_nodes.committed);

    let current_majority = leader.record_acks(second.index, BTreeSet::from([NodeId(1), NodeId(2)]));
    assert!(current_majority.committed);
}

#[test]
fn membership_reconfiguration_rejects_empty_or_local_removal() {
    let mut leader = ConsensusState::new(NodeId(1), voters());
    assert!(leader.reconfigure_voters(BTreeSet::new()).is_err());
    assert!(leader
        .reconfigure_voters(BTreeSet::from([NodeId(2), NodeId(3)]))
        .is_err());

    let mut follower = ElectionState::new(NodeId(2), voters());
    assert!(follower.reconfigure_voters(BTreeSet::new()).is_err());
    assert!(follower
        .reconfigure_voters(BTreeSet::from([NodeId(1), NodeId(3)]))
        .is_err());
}

fn voters() -> BTreeSet<NodeId> {
    BTreeSet::from([NodeId(1), NodeId(2), NodeId(3)])
}
