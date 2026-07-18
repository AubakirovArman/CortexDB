#![cfg(feature = "experimental-replication")]

use std::collections::{BTreeMap, BTreeSet};

use cortex_engine::{
    decode_joint_membership_entry, joint_membership_entry, plan_replication_repair_sweep,
    repair_lagging_voter, resume_joint_membership_rotation, AppendEntriesRequest, ConsensusState,
    ElectionState, InMemoryReplicationTransport, LogIndex, NodeId, ReplicationFollowerProgress,
    ReplicationRecoveryPolicy, ReplicationRepairDecisionKind, Term,
};

#[test]
fn split_brain_rejoin_repair_soak_keeps_commits_majority_only() {
    let voters = BTreeSet::from([NodeId(1), NodeId(2), NodeId(3), NodeId(4), NodeId(5)]);
    let mut leader = ConsensusState::new(NodeId(1), voters.clone());
    let mut transport = transport_with_peers(&voters);

    for round in 1..=12 {
        let entry = leader.append_local(format!("round {round}").into_bytes());
        transport.set_partitions(&[
            BTreeSet::from([NodeId(1), NodeId(2)]),
            BTreeSet::from([NodeId(3), NodeId(4), NodeId(5)]),
        ]);
        let minority_acks = transport
            .replicate_to_best_effort(
                [NodeId(2), NodeId(3), NodeId(4), NodeId(5)],
                append_all_request(&leader, LogIndex(0)),
            )
            .unwrap();
        assert!(!leader.record_acks(entry.index, minority_acks).committed);

        transport.heal_partitions();
        let healed_acks = transport
            .replicate_to_best_effort(
                [NodeId(2), NodeId(3)],
                append_all_request(&leader, leader.commit_index),
            )
            .unwrap();
        assert!(leader.record_acks(entry.index, healed_acks).committed);
    }

    for target in [NodeId(4), NodeId(5)] {
        let result = repair_lagging_voter(
            &leader,
            &mut transport,
            target,
            LogIndex(0),
            ReplicationRecoveryPolicy {
                snapshot_threshold: 20,
            },
        )
        .unwrap();
        assert!(result.success);
        assert!(result.append_sent);
        assert_eq!(transport.peer_commit(target), Some(leader.commit_index));
        assert_eq!(transport.peer_log(target).unwrap().len(), 12);
    }
}

#[test]
fn follower_lag_repair_soak_escalates_to_snapshot_then_returns_idle() {
    let voters = BTreeSet::from([NodeId(1), NodeId(2), NodeId(3), NodeId(4)]);
    let mut leader = ConsensusState::new(NodeId(1), voters);
    for round in 1..=8 {
        let entry = leader.append_local(format!("committed {round}").into_bytes());
        assert!(
            leader
                .record_acks(
                    entry.index,
                    BTreeSet::from([NodeId(1), NodeId(2), NodeId(3)])
                )
                .committed
        );
    }

    let policy = ReplicationRecoveryPolicy {
        snapshot_threshold: 3,
    };
    let schedule = plan_replication_repair_sweep(
        &leader,
        &BTreeMap::from([
            (
                NodeId(2),
                ReplicationFollowerProgress::new(NodeId(2), LogIndex(8), LogIndex(8)),
            ),
            (
                NodeId(3),
                ReplicationFollowerProgress::new(NodeId(3), LogIndex(6), LogIndex(6)),
            ),
            (
                NodeId(4),
                ReplicationFollowerProgress::new(NodeId(4), LogIndex(0), LogIndex(0)),
            ),
        ]),
        policy,
    )
    .unwrap();
    assert_eq!(schedule.already_caught_up_count(), 1);
    assert_eq!(schedule.append_entries_count(), 1);
    assert_eq!(schedule.snapshot_required_count(), 1);
    assert_eq!(
        schedule.decisions[2].kind,
        ReplicationRepairDecisionKind::InstallSnapshot
    );

    let idle = plan_replication_repair_sweep(
        &leader,
        &BTreeMap::from([
            (
                NodeId(2),
                ReplicationFollowerProgress::new(NodeId(2), LogIndex(8), LogIndex(8)),
            ),
            (
                NodeId(3),
                ReplicationFollowerProgress::new(NodeId(3), LogIndex(8), LogIndex(8)),
            ),
            (
                NodeId(4),
                ReplicationFollowerProgress::new(NodeId(4), LogIndex(8), LogIndex(8)),
            ),
        ]),
        policy,
    )
    .unwrap();
    assert!(idle.is_idle());
}

#[test]
fn membership_rotation_resume_can_continue_to_next_rotation() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("replication.aclog");
    let initial = BTreeSet::from([NodeId(1), NodeId(2), NodeId(3)]);
    let first = BTreeSet::from([NodeId(1), NodeId(2), NodeId(3), NodeId(4)]);
    let second = BTreeSet::from([NodeId(1), NodeId(3), NodeId(4), NodeId(5)]);
    let mut leader = ConsensusState::new(NodeId(1), initial.clone());
    let log = cortex_engine::ReplicationLog::open(&path).unwrap();

    let joint =
        joint_membership_entry(leader.current_term, LogIndex(1), initial, first.clone()).unwrap();
    let joint = leader.append_local(joint.payload);
    log.append(&joint).unwrap();
    let joint_config = decode_joint_membership_entry(&joint).unwrap().unwrap();
    assert!(
        leader
            .record_joint_consensus_acks(
                joint.index,
                &joint_config,
                BTreeSet::from([NodeId(1), NodeId(2), NodeId(3), NodeId(4)]),
            )
            .committed
    );
    log.close().unwrap();

    let mut recovered = cortex_engine::ReplicationLog::recover_consensus_with_membership(
        &path,
        NodeId(1),
        BTreeSet::from([NodeId(1), NodeId(2), NodeId(3)]),
        LogIndex(1),
    )
    .unwrap();
    let mut transport = transport_with_peers(&BTreeSet::from([
        NodeId(1),
        NodeId(2),
        NodeId(3),
        NodeId(4),
        NodeId(5),
    ]));
    let log = cortex_engine::ReplicationLog::open(&path).unwrap();
    let resumed =
        resume_joint_membership_rotation(&mut recovered, &mut transport, &log, joint_config)
            .unwrap();
    assert_eq!(resumed.final_voters, first);

    let second_joint =
        joint_membership_entry(recovered.current_term, LogIndex(3), first, second.clone()).unwrap();
    let second_joint = recovered.append_local(second_joint.payload);
    log.append(&second_joint).unwrap();
    let second_joint_config = decode_joint_membership_entry(&second_joint)
        .unwrap()
        .unwrap();
    assert!(
        recovered
            .record_joint_consensus_acks(
                second_joint.index,
                &second_joint_config,
                BTreeSet::from([NodeId(1), NodeId(3), NodeId(4)]),
            )
            .committed
    );
    log.close().unwrap();
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

fn append_all_request(leader: &ConsensusState, leader_commit: LogIndex) -> AppendEntriesRequest {
    AppendEntriesRequest {
        term: leader.current_term,
        leader_id: leader.local_node,
        prev_log_index: LogIndex(0),
        prev_log_term: Term(0),
        entries: leader.entries().to_vec(),
        leader_commit,
    }
}
