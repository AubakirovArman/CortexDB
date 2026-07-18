#![cfg(feature = "experimental-replication")]

use std::collections::BTreeSet;

use cortex_engine::{
    decode_joint_membership_entry, decode_membership_entry, joint_membership_entry,
    membership_entry, recover_membership_config, recover_voting_config,
    rotate_membership_with_joint_consensus, ConsensusState, ElectionState,
    InMemoryReplicationTransport, LogIndex, MembershipConfig, MembershipRotationPhase, NodeId,
    ReplicatedEntry, ReplicationLog, Term, VotingConfig,
};

#[test]
fn membership_entry_roundtrips_voters_deterministically() {
    let voters = BTreeSet::from([NodeId(3), NodeId(1), NodeId(2)]);
    let entry = membership_entry(Term(4), LogIndex(7), voters.clone()).unwrap();

    assert_eq!(entry.term, Term(4));
    assert_eq!(entry.index, LogIndex(7));
    assert_eq!(
        decode_membership_entry(&entry).unwrap().unwrap().voters,
        voters
    );
}

#[test]
fn membership_recovery_ignores_uncommitted_rotation() {
    let initial = BTreeSet::from([NodeId(1), NodeId(2), NodeId(3)]);
    let committed = membership_entry(
        Term(2),
        LogIndex(2),
        BTreeSet::from([NodeId(1), NodeId(2), NodeId(3), NodeId(4), NodeId(5)]),
    )
    .unwrap();
    let uncommitted = membership_entry(Term(3), LogIndex(3), BTreeSet::from([NodeId(1)])).unwrap();

    let config =
        recover_membership_config(&[committed.clone(), uncommitted], initial, LogIndex(2)).unwrap();
    assert_eq!(
        config.voters,
        decode_membership_entry(&committed).unwrap().unwrap().voters
    );
}

#[test]
fn membership_rotation_survives_replication_log_restart() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("replication.aclog");
    let initial = BTreeSet::from([NodeId(1), NodeId(2), NodeId(3)]);
    let rotated = BTreeSet::from([NodeId(1), NodeId(2), NodeId(3), NodeId(4), NodeId(5)]);
    let entry = membership_entry(Term(2), LogIndex(1), rotated.clone()).unwrap();

    {
        let log = ReplicationLog::open(&path).unwrap();
        log.append(&entry).unwrap();
        log.close().unwrap();
    }

    let recovered =
        ReplicationLog::recover_consensus_with_membership(&path, NodeId(1), initial, LogIndex(1))
            .unwrap();
    assert_eq!(recovered.voters, rotated);

    let mut resumed = recovered;
    let next = resumed.append_local(b"after membership".to_vec());
    assert_eq!(next.index, LogIndex(2));
}

#[test]
fn non_membership_entries_do_not_change_config() {
    let initial = BTreeSet::from([NodeId(1), NodeId(2), NodeId(3)]);
    let entry = ReplicatedEntry {
        term: Term(1),
        index: LogIndex(1),
        payload: b"ordinary operation".to_vec(),
    };

    assert!(decode_membership_entry(&entry).unwrap().is_none());
    let config = recover_membership_config(&[entry], initial.clone(), LogIndex(1)).unwrap();
    assert_eq!(config.voters, initial);
}

#[test]
fn empty_membership_is_rejected() {
    assert!(MembershipConfig::new(BTreeSet::new()).is_err());
    assert!(membership_entry(Term(1), LogIndex(1), BTreeSet::new()).is_err());
    assert!(joint_membership_entry(
        Term(1),
        LogIndex(1),
        BTreeSet::new(),
        BTreeSet::from([NodeId(1)])
    )
    .is_err());
}

#[test]
fn joint_membership_entry_roundtrips_old_and_new_voters() {
    let old = BTreeSet::from([NodeId(1), NodeId(2), NodeId(3)]);
    let new = BTreeSet::from([NodeId(3), NodeId(4), NodeId(5)]);
    let entry = joint_membership_entry(Term(5), LogIndex(9), old.clone(), new.clone()).unwrap();
    let decoded = decode_joint_membership_entry(&entry).unwrap().unwrap();

    assert_eq!(entry.term, Term(5));
    assert_eq!(entry.index, LogIndex(9));
    assert_eq!(decoded.old_voters, old);
    assert_eq!(decoded.new_voters, new);
    assert!(decode_membership_entry(&entry).unwrap().is_none());
}

#[test]
fn joint_consensus_requires_old_and_new_majorities() {
    let old = BTreeSet::from([NodeId(1), NodeId(2), NodeId(3)]);
    let new = BTreeSet::from([NodeId(3), NodeId(4), NodeId(5)]);
    let joint = decode_joint_membership_entry(
        &joint_membership_entry(Term(1), LogIndex(1), old, new).unwrap(),
    )
    .unwrap()
    .unwrap();
    let mut leader =
        ConsensusState::new(NodeId(1), BTreeSet::from([NodeId(1), NodeId(2), NodeId(3)]));
    let entry = leader.append_local(b"joint config".to_vec());

    let old_only = leader.record_joint_consensus_acks(
        entry.index,
        &joint,
        BTreeSet::from([NodeId(1), NodeId(2)]),
    );
    assert!(!old_only.committed);
    assert_eq!(old_only.old_acknowledgements, 2);
    assert_eq!(old_only.new_acknowledgements, 0);

    let new_only = leader.record_joint_consensus_acks(
        entry.index,
        &joint,
        BTreeSet::from([NodeId(3), NodeId(4)]),
    );
    assert!(!new_only.committed);
    assert_eq!(new_only.old_acknowledgements, 1);
    assert_eq!(new_only.new_acknowledgements, 2);

    let both = leader.record_joint_consensus_acks(
        entry.index,
        &joint,
        BTreeSet::from([NodeId(1), NodeId(2), NodeId(3), NodeId(4)]),
    );
    assert!(both.committed);
    assert_eq!(leader.commit_index, entry.index);
}

#[test]
fn recover_voting_config_preserves_latest_committed_joint_config() {
    let initial = BTreeSet::from([NodeId(1), NodeId(2), NodeId(3)]);
    let old = initial.clone();
    let new = BTreeSet::from([NodeId(3), NodeId(4), NodeId(5)]);
    let joint = joint_membership_entry(Term(2), LogIndex(2), old.clone(), new.clone()).unwrap();
    let uncommitted_stable =
        membership_entry(Term(3), LogIndex(3), BTreeSet::from([NodeId(4), NodeId(5)])).unwrap();

    let recovered =
        recover_voting_config(&[joint, uncommitted_stable], initial, LogIndex(2)).unwrap();
    assert_eq!(
        recovered,
        VotingConfig::Joint(cortex_engine::JointMembershipConfig::new(old, new).unwrap())
    );
}

#[test]
fn stable_membership_after_joint_config_becomes_effective() {
    let initial = BTreeSet::from([NodeId(1), NodeId(2), NodeId(3)]);
    let old = initial.clone();
    let new = BTreeSet::from([NodeId(3), NodeId(4), NodeId(5)]);
    let joint = joint_membership_entry(Term(2), LogIndex(2), old, new.clone()).unwrap();
    let stable = membership_entry(Term(3), LogIndex(3), new.clone()).unwrap();

    let recovered = recover_voting_config(&[joint, stable], initial, LogIndex(3)).unwrap();
    assert_eq!(
        recovered,
        VotingConfig::Stable(MembershipConfig::new(new).unwrap())
    );
}

#[test]
fn recover_consensus_with_committed_joint_config_uses_union_voters() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("replication.aclog");
    let initial = BTreeSet::from([NodeId(1), NodeId(2), NodeId(3)]);
    let old = initial.clone();
    let new = BTreeSet::from([NodeId(3), NodeId(4), NodeId(5)]);
    let entry = joint_membership_entry(Term(2), LogIndex(1), old, new).unwrap();

    {
        let log = ReplicationLog::open(&path).unwrap();
        log.append(&entry).unwrap();
        log.close().unwrap();
    }

    let recovered =
        ReplicationLog::recover_consensus_with_membership(&path, NodeId(1), initial, LogIndex(1))
            .unwrap();
    assert_eq!(
        recovered.voters,
        BTreeSet::from([NodeId(1), NodeId(2), NodeId(3), NodeId(4), NodeId(5)])
    );
}

#[test]
fn automated_membership_rotation_commits_joint_then_stable_config() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("replication.aclog");
    let initial = BTreeSet::from([NodeId(1), NodeId(2), NodeId(3)]);
    let rotated = BTreeSet::from([NodeId(1), NodeId(3), NodeId(4), NodeId(5)]);
    let mut leader = ConsensusState::new(NodeId(1), initial.clone());
    let mut transport = transport_with_peers(&BTreeSet::from([
        NodeId(1),
        NodeId(2),
        NodeId(3),
        NodeId(4),
        NodeId(5),
    ]));
    let log = ReplicationLog::open(&path).unwrap();
    let seed = leader.append_local(b"existing committed entry".to_vec());
    log.append(&seed).unwrap();
    assert!(
        leader
            .record_acks(seed.index, BTreeSet::from([NodeId(1), NodeId(2)]))
            .committed
    );

    let result =
        rotate_membership_with_joint_consensus(&mut leader, &mut transport, &log, rotated.clone())
            .unwrap();
    log.close().unwrap();
    let recovered =
        ReplicationLog::recover_consensus_with_membership(&path, NodeId(1), initial, LogIndex(3))
            .unwrap();

    assert_eq!(result.phase, MembershipRotationPhase::Complete);
    assert_eq!(result.joint_entry.index, LogIndex(2));
    assert_eq!(result.stable_entry.unwrap().index, LogIndex(3));
    assert_eq!(result.final_voters, rotated);
    assert_eq!(leader.voters, result.final_voters);
    assert_eq!(leader.commit_index, LogIndex(3));
    assert_eq!(recovered.voters, leader.voters);
    assert_eq!(transport.peer_commit(NodeId(4)), Some(LogIndex(2)));
    assert_eq!(transport.peer_log(NodeId(4)).unwrap().len(), 3);
}

#[test]
fn automated_membership_rotation_does_not_publish_stable_config_without_joint_quorum() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("replication.aclog");
    let initial = BTreeSet::from([NodeId(1), NodeId(2), NodeId(3)]);
    let rotated = BTreeSet::from([NodeId(1), NodeId(4), NodeId(5)]);
    let mut leader = ConsensusState::new(NodeId(1), initial.clone());
    let mut transport = transport_with_peers(&BTreeSet::from([
        NodeId(1),
        NodeId(2),
        NodeId(3),
        NodeId(4),
        NodeId(5),
    ]));
    transport.set_partitions(&[
        BTreeSet::from([NodeId(1), NodeId(2)]),
        BTreeSet::from([NodeId(3), NodeId(4), NodeId(5)]),
    ]);
    let log = ReplicationLog::open(&path).unwrap();

    let result =
        rotate_membership_with_joint_consensus(&mut leader, &mut transport, &log, rotated).unwrap();
    log.close().unwrap();
    let entries = ReplicationLog::recover_entries(&path).unwrap();
    let recovered =
        ReplicationLog::recover_consensus_with_membership(&path, NodeId(1), initial, LogIndex(0))
            .unwrap();

    assert_eq!(result.phase, MembershipRotationPhase::JointNotCommitted);
    assert!(result.stable_entry.is_none());
    assert_eq!(result.joint_decision.old_acknowledgements, 2);
    assert_eq!(result.joint_decision.new_acknowledgements, 1);
    assert_eq!(leader.commit_index, LogIndex(0));
    assert_eq!(entries.len(), 1);
    assert_eq!(
        recovered.voters,
        BTreeSet::from([NodeId(1), NodeId(2), NodeId(3)])
    );
}

fn transport_with_peers(nodes: &BTreeSet<NodeId>) -> InMemoryReplicationTransport {
    let mut transport = InMemoryReplicationTransport::default();
    for node in nodes {
        let mut state = ElectionState::new(*node, nodes.clone());
        state.current_term = Term(1);
        transport.register_peer(state);
    }
    transport
}
