#![cfg(feature = "experimental-replication")]

use std::collections::{BTreeMap, BTreeSet};

use cortex_engine::{
    execute_replication_repair_schedule, run_replication_repair_cycle, AppendEntriesRequest,
    ConsensusState, ElectionState, InMemoryReplicationTransport, LogIndex, NodeId, ReplicatedEntry,
    ReplicationFollowerProgress, ReplicationRecoveryAction, ReplicationRecoveryPlan,
    ReplicationRecoveryPolicy, ReplicationRepairDecision, ReplicationRepairDecisionKind,
    ReplicationRepairSchedule, ReplicationTransport, Term,
};

#[test]
fn repair_cycle_executes_append_and_returns_snapshot_requests() {
    let voters = BTreeSet::from([NodeId(1), NodeId(2), NodeId(3), NodeId(4)]);
    let leader = committed_leader(&voters, 5);
    let entries = leader.entries().to_vec();
    let mut transport = transport_with_followers(&voters);
    seed_follower(&mut transport, &leader, NodeId(2), &entries, LogIndex(5));
    seed_follower(
        &mut transport,
        &leader,
        NodeId(3),
        &entries[..3],
        LogIndex(3),
    );

    let result = run_replication_repair_cycle(
        &leader,
        &mut transport,
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
        ]),
        ReplicationRecoveryPolicy {
            snapshot_threshold: 3,
        },
    )
    .unwrap();

    assert_eq!(result.already_caught_up_count(), 1);
    assert_eq!(result.repaired_count(), 1);
    assert_eq!(result.snapshot_required_count(), 1);
    assert_eq!(transport.peer_commit(NodeId(2)), Some(LogIndex(5)));
    assert_eq!(transport.peer_commit(NodeId(3)), Some(LogIndex(5)));
    assert!(transport.peer_log(NodeId(4)).unwrap().is_empty());
    assert_eq!(result.snapshot_requests[0].target, NodeId(4));
    assert_eq!(result.snapshot_requests[0].checkpoint, LogIndex(5));
}

#[test]
fn repair_cycle_rejects_schedule_that_disagrees_with_plan() {
    let voters = BTreeSet::from([NodeId(1), NodeId(2)]);
    let leader = committed_leader(&voters, 1);
    let mut transport = transport_with_followers(&voters);
    let schedule = ReplicationRepairSchedule {
        decisions: vec![ReplicationRepairDecision {
            target: NodeId(2),
            plan: ReplicationRecoveryPlan {
                lag: 0,
                action: ReplicationRecoveryAction::AlreadyCaughtUp,
            },
            kind: ReplicationRepairDecisionKind::AppendEntries,
        }],
    };

    let result = execute_replication_repair_schedule(&leader, &mut transport, &schedule);

    assert!(result.is_err());
}

fn committed_leader(voters: &BTreeSet<NodeId>, entries: usize) -> ConsensusState {
    let mut leader = ConsensusState::new(NodeId(1), voters.clone());
    for index in 0..entries {
        let entry = leader.append_local(format!("entry {index}").into_bytes());
        leader.record_acks(
            entry.index,
            BTreeSet::from([NodeId(1), NodeId(2), NodeId(3)]),
        );
    }
    leader
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
    entries: &[ReplicatedEntry],
    leader_commit: LogIndex,
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
}
