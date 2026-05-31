use std::collections::{BTreeMap, BTreeSet};

use cortex_engine::{
    plan_replication_repair_sweep, repair_lagging_voter, repair_lagging_voters,
    AppendEntriesRequest, ConsensusState, ElectionState, InMemoryReplicationTransport, LogIndex,
    NodeId, ReplicationFollowerProgress, ReplicationRecoveryAction, ReplicationRecoveryPolicy,
    ReplicationRepairDecisionKind, ReplicationTransport, Term,
};

#[test]
fn rejoined_voter_catches_up_with_missing_entries_after_partition() {
    let voters = BTreeSet::from([NodeId(1), NodeId(2), NodeId(3), NodeId(4), NodeId(5)]);
    let mut leader = ConsensusState::new(NodeId(1), voters.clone());
    let entry1 = leader.append_local(b"before partition".to_vec());
    let entry2 = leader.append_local(b"committed while follower partitioned".to_vec());
    let entry3 = leader.append_local(b"second committed while follower partitioned".to_vec());
    leader.record_acks(
        entry1.index,
        BTreeSet::from([NodeId(1), NodeId(2), NodeId(3)]),
    );
    leader.record_acks(
        entry2.index,
        BTreeSet::from([NodeId(1), NodeId(2), NodeId(3)]),
    );
    leader.record_acks(
        entry3.index,
        BTreeSet::from([NodeId(1), NodeId(2), NodeId(3)]),
    );

    let mut transport = transport_with_followers(&voters);
    transport
        .append_entries(
            NodeId(5),
            AppendEntriesRequest {
                term: leader.current_term,
                leader_id: leader.local_node,
                prev_log_index: LogIndex(0),
                prev_log_term: Term(0),
                entries: vec![entry1],
                leader_commit: LogIndex(1),
            },
        )
        .unwrap();

    let result = repair_lagging_voter(
        &leader,
        &mut transport,
        NodeId(5),
        LogIndex(1),
        ReplicationRecoveryPolicy {
            snapshot_threshold: 10,
        },
    )
    .unwrap();

    assert!(result.success);
    assert!(result.append_sent);
    assert_eq!(result.plan.lag, 2);
    assert_eq!(transport.peer_log(NodeId(5)).unwrap().len(), 3);
    assert_eq!(transport.peer_commit(NodeId(5)), Some(LogIndex(3)));
}

#[test]
fn repair_selects_snapshot_without_append_when_lag_exceeds_threshold() {
    let voters = BTreeSet::from([NodeId(1), NodeId(2), NodeId(3)]);
    let mut leader = ConsensusState::new(NodeId(1), voters.clone());
    for index in 0..4 {
        let entry = leader.append_local(format!("entry {index}").into_bytes());
        leader.record_acks(entry.index, BTreeSet::from([NodeId(1), NodeId(2)]));
    }
    let mut transport = transport_with_followers(&voters);

    let result = repair_lagging_voter(
        &leader,
        &mut transport,
        NodeId(3),
        LogIndex(0),
        ReplicationRecoveryPolicy {
            snapshot_threshold: 2,
        },
    )
    .unwrap();

    assert!(!result.success);
    assert!(!result.append_sent);
    assert_eq!(
        result.plan.action,
        ReplicationRecoveryAction::InstallSnapshot {
            checkpoint: LogIndex(4)
        }
    );
    assert!(transport.peer_log(NodeId(3)).unwrap().is_empty());
}

#[test]
fn repair_rejects_non_voter_target() {
    let voters = BTreeSet::from([NodeId(1), NodeId(2), NodeId(3)]);
    let leader = ConsensusState::new(NodeId(1), voters.clone());
    let mut transport = transport_with_followers(&voters);

    let result = repair_lagging_voter(
        &leader,
        &mut transport,
        NodeId(4),
        LogIndex(0),
        ReplicationRecoveryPolicy::default(),
    );

    assert!(result.is_err());
}

#[test]
fn repair_sweep_handles_caught_up_lagging_and_snapshot_voters() {
    let voters = BTreeSet::from([NodeId(1), NodeId(2), NodeId(3), NodeId(4)]);
    let mut leader = ConsensusState::new(NodeId(1), voters.clone());
    let mut entries = Vec::new();
    for index in 0..5 {
        let entry = leader.append_local(format!("entry {index}").into_bytes());
        leader.record_acks(
            entry.index,
            BTreeSet::from([NodeId(1), NodeId(2), NodeId(3)]),
        );
        entries.push(entry);
    }
    let mut transport = transport_with_followers(&voters);
    seed_follower(&mut transport, &leader, NodeId(2), &entries, LogIndex(5), 5);
    seed_follower(
        &mut transport,
        &leader,
        NodeId(3),
        &entries[..3],
        LogIndex(3),
        3,
    );

    let result = repair_lagging_voters(
        &leader,
        &mut transport,
        &BTreeMap::from([
            (NodeId(2), LogIndex(5)),
            (NodeId(3), LogIndex(3)),
            (NodeId(4), LogIndex(0)),
            (NodeId(99), LogIndex(5)),
        ]),
        ReplicationRecoveryPolicy {
            snapshot_threshold: 3,
        },
    )
    .unwrap();

    assert_eq!(result.results.len(), 3);
    assert_eq!(result.already_caught_up_count(), 1);
    assert_eq!(result.repaired_count(), 1);
    assert_eq!(result.snapshot_required_count(), 1);
    assert_eq!(transport.peer_commit(NodeId(2)), Some(LogIndex(5)));
    assert_eq!(transport.peer_commit(NodeId(3)), Some(LogIndex(5)));
    assert!(transport.peer_log(NodeId(4)).unwrap().is_empty());
}

#[test]
fn repair_schedule_classifies_voters_from_progress() {
    let voters = BTreeSet::from([NodeId(1), NodeId(2), NodeId(3), NodeId(4)]);
    let mut leader = ConsensusState::new(NodeId(1), voters);
    for index in 0..5 {
        let entry = leader.append_local(format!("entry {index}").into_bytes());
        leader.record_acks(
            entry.index,
            BTreeSet::from([NodeId(1), NodeId(2), NodeId(3)]),
        );
    }

    let schedule = plan_replication_repair_sweep(
        &leader,
        &BTreeMap::from([
            (
                NodeId(2),
                ReplicationFollowerProgress::new(NodeId(2), LogIndex(5), LogIndex(5)),
            ),
            (
                NodeId(3),
                ReplicationFollowerProgress::new(NodeId(3), LogIndex(3), LogIndex(3)),
            ),
            (
                NodeId(4),
                ReplicationFollowerProgress::new(NodeId(4), LogIndex(0), LogIndex(0)),
            ),
            (
                NodeId(99),
                ReplicationFollowerProgress::new(NodeId(99), LogIndex(5), LogIndex(5)),
            ),
        ]),
        ReplicationRecoveryPolicy {
            snapshot_threshold: 3,
        },
    )
    .unwrap();

    assert_eq!(schedule.decisions.len(), 3);
    assert_eq!(schedule.already_caught_up_count(), 1);
    assert_eq!(schedule.append_entries_count(), 1);
    assert_eq!(schedule.snapshot_required_count(), 1);
    assert_eq!(
        schedule
            .decisions
            .iter()
            .map(|decision| decision.target)
            .collect::<Vec<_>>(),
        vec![NodeId(2), NodeId(3), NodeId(4)]
    );
    assert_eq!(
        schedule.decisions[0].kind,
        ReplicationRepairDecisionKind::AlreadyCaughtUp
    );
    assert_eq!(
        schedule.decisions[1].kind,
        ReplicationRepairDecisionKind::AppendEntries
    );
    assert_eq!(
        schedule.decisions[2].kind,
        ReplicationRepairDecisionKind::InstallSnapshot
    );
}

#[test]
fn repair_schedule_rejects_inconsistent_progress() {
    let voters = BTreeSet::from([NodeId(1), NodeId(2)]);
    let mut leader = ConsensusState::new(NodeId(1), voters);
    let entry = leader.append_local(b"entry".to_vec());
    leader.record_acks(entry.index, BTreeSet::from([NodeId(1), NodeId(2)]));

    for invalid_progress in [
        ReplicationFollowerProgress::new(NodeId(99), LogIndex(1), LogIndex(1)),
        ReplicationFollowerProgress::new(NodeId(2), LogIndex(2), LogIndex(2)),
        ReplicationFollowerProgress::new(NodeId(2), LogIndex(1), LogIndex(0)),
    ] {
        let result = plan_replication_repair_sweep(
            &leader,
            &BTreeMap::from([(NodeId(2), invalid_progress)]),
            ReplicationRecoveryPolicy::default(),
        );
        assert!(result.is_err());
    }
}

fn transport_with_followers(voters: &BTreeSet<NodeId>) -> InMemoryReplicationTransport {
    let mut transport = InMemoryReplicationTransport::default();
    for node in voters.iter().copied().filter(|node| *node != NodeId(1)) {
        let mut state = ElectionState::new(node, voters.clone());
        state.current_term = Term(1);
        transport.register_peer(state);
    }
    transport
}

fn seed_follower(
    transport: &mut InMemoryReplicationTransport,
    leader: &ConsensusState,
    target: NodeId,
    entries: &[cortex_engine::ReplicatedEntry],
    leader_commit: LogIndex,
    expected_len: usize,
) {
    transport
        .append_entries(
            target,
            AppendEntriesRequest {
                term: leader.current_term,
                leader_id: leader.local_node,
                prev_log_index: LogIndex(0),
                prev_log_term: Term(0),
                entries: entries.to_vec(),
                leader_commit,
            },
        )
        .unwrap();
    assert_eq!(transport.peer_log(target).unwrap().len(), expected_len);
}
