use std::collections::BTreeSet;

use cortex_engine::{
    decode_joint_membership_entry, joint_membership_entry, resume_joint_membership_rotation,
    ConsensusState, ElectionState, InMemoryReplicationTransport, LogIndex, MembershipRotationPhase,
    NodeId, ReplicationLog, Term,
};

#[test]
fn membership_rotation_resumes_after_restart_with_committed_joint_config() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("replication.aclog");
    let initial = BTreeSet::from([NodeId(1), NodeId(2), NodeId(3)]);
    let rotated = BTreeSet::from([NodeId(1), NodeId(3), NodeId(4), NodeId(5)]);
    let mut leader = ConsensusState::new(NodeId(1), initial.clone());
    let log = ReplicationLog::open(&path).unwrap();
    let seed = leader.append_local(b"committed before rotation".to_vec());
    log.append(&seed).unwrap();
    assert!(
        leader
            .record_acks(seed.index, BTreeSet::from([NodeId(1), NodeId(2)]))
            .committed
    );
    let joint_entry = joint_membership_entry(
        leader.current_term,
        LogIndex(2),
        initial.clone(),
        rotated.clone(),
    )
    .unwrap();
    let joint_entry = leader.append_local(joint_entry.payload);
    log.append(&joint_entry).unwrap();
    let joint_config = decode_joint_membership_entry(&joint_entry)
        .unwrap()
        .unwrap();
    let joint_decision = leader.record_joint_consensus_acks(
        joint_entry.index,
        &joint_config,
        BTreeSet::from([NodeId(1), NodeId(2), NodeId(3), NodeId(4)]),
    );
    assert!(joint_decision.committed);
    log.close().unwrap();

    let mut recovered =
        ReplicationLog::recover_consensus_with_membership(&path, NodeId(1), initial, LogIndex(2))
            .unwrap();
    let mut transport = transport_with_peers(&BTreeSet::from([
        NodeId(1),
        NodeId(2),
        NodeId(3),
        NodeId(4),
        NodeId(5),
    ]));
    let log = ReplicationLog::open(&path).unwrap();
    let result =
        resume_joint_membership_rotation(&mut recovered, &mut transport, &log, joint_config)
            .unwrap();
    log.close().unwrap();
    let final_state = ReplicationLog::recover_consensus_with_membership(
        &path,
        NodeId(1),
        BTreeSet::from([NodeId(1), NodeId(2), NodeId(3)]),
        LogIndex(3),
    )
    .unwrap();

    assert_eq!(result.phase, MembershipRotationPhase::Complete);
    assert_eq!(result.stable_entry.index, LogIndex(3));
    assert_eq!(result.final_voters, rotated);
    assert_eq!(recovered.voters, result.final_voters);
    assert_eq!(recovered.commit_index, LogIndex(3));
    assert_eq!(final_state.voters, result.final_voters);
    assert_eq!(transport.peer_log(NodeId(5)).unwrap().len(), 3);
}

#[test]
fn membership_rotation_resume_requires_committed_joint_config() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("replication.aclog");
    let initial = BTreeSet::from([NodeId(1), NodeId(2), NodeId(3)]);
    let rotated = BTreeSet::from([NodeId(1), NodeId(3), NodeId(4), NodeId(5)]);
    let mut leader = ConsensusState::new(NodeId(1), initial.clone());
    let log = ReplicationLog::open(&path).unwrap();
    let joint_entry =
        joint_membership_entry(leader.current_term, LogIndex(1), initial.clone(), rotated).unwrap();
    let joint_entry = leader.append_local(joint_entry.payload);
    log.append(&joint_entry).unwrap();
    let joint_config = decode_joint_membership_entry(&joint_entry)
        .unwrap()
        .unwrap();
    log.close().unwrap();

    let mut recovered =
        ReplicationLog::recover_consensus_with_membership(&path, NodeId(1), initial, LogIndex(0))
            .unwrap();
    let mut transport = transport_with_peers(&BTreeSet::from([
        NodeId(1),
        NodeId(2),
        NodeId(3),
        NodeId(4),
    ]));
    let log = ReplicationLog::open(&path).unwrap();
    let result =
        resume_joint_membership_rotation(&mut recovered, &mut transport, &log, joint_config);
    log.close().unwrap();

    assert!(result.is_err());
    assert_eq!(recovered.commit_index, LogIndex(0));
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
