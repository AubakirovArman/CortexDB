use std::collections::{BTreeMap, BTreeSet};

use cortex_engine::{
    AppendEntriesRequest, ConsensusState, ElectionRole, ElectionState,
    InMemoryReplicationTransport, LogIndex, NodeId, ReplicationLog, ReplicationTransport, Term,
};

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

fn voters() -> BTreeSet<NodeId> {
    BTreeSet::from([NodeId(1), NodeId(2), NodeId(3)])
}
