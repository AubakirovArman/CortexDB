#[path = "cluster_fail_closed/support.rs"]
mod support;

use std::collections::BTreeSet;

use cortex_core::CellId;
use cortex_engine::{
    ConsensusState, ElectionRole, ElectionState, LogIndex, NodeId, ReplicationTransport, Term,
};

use support::{
    append_request, assert_same_pack_and_receipt, checked_evidence, five_voters,
    install_snapshot_over_peer, open_replication_db, private_ready, ready_project,
    seed_committed_cells, seed_stale_follower, transport_with_followers,
};

#[test]
fn follower_read_network_snapshot_preserves_fail_closed_receipt() {
    let leader_dir = tempfile::tempdir().unwrap();
    let follower_dir = tempfile::tempdir().unwrap();
    let mut leader = open_replication_db(leader_dir.path());
    seed_committed_cells(&mut leader);
    seed_stale_follower(follower_dir.path());

    let leader_evidence = checked_evidence(&leader, &[CellId(1), CellId(2)]);
    install_snapshot_over_peer(
        follower_dir.path(),
        leader.replication_snapshot_segment().unwrap(),
    );
    let follower = open_replication_db(follower_dir.path());
    let follower_evidence = checked_evidence(&follower, &[CellId(1), CellId(2)]);

    assert_same_pack_and_receipt(&leader_evidence, &follower_evidence);
}

#[test]
fn failover_read_rejects_stale_old_leader_widening() {
    let leader_dir = tempfile::tempdir().unwrap();
    let new_leader_dir = tempfile::tempdir().unwrap();
    let mut old_leader_db = open_replication_db(leader_dir.path());
    seed_committed_cells(&mut old_leader_db);
    let committed_evidence = checked_evidence(&old_leader_db, &[CellId(1), CellId(2)]);
    install_snapshot_over_peer(
        new_leader_dir.path(),
        old_leader_db.replication_snapshot_segment().unwrap(),
    );

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
    assert!(
        !candidate
            .record_vote(
                transport
                    .request_vote(NodeId(4), vote_request.clone())
                    .unwrap()
            )
            .elected
    );
    let outcome = candidate.record_vote(transport.request_vote(NodeId(5), vote_request).unwrap());
    assert!(outcome.elected);
    assert_eq!(outcome.role, ElectionRole::Leader);

    old_leader_db
        .put_cell(
            CellId(50),
            ready_project("old-leader", "stale old leader widened answer"),
        )
        .unwrap();
    old_leader_db
        .put_cell(
            CellId(51),
            private_ready("old-leader", "stale old leader private answer"),
        )
        .unwrap();
    transport.heal_partitions();
    let mut old_leader = ConsensusState::new(NodeId(1), voters);
    old_leader.current_term = Term(1);
    let stale_entry = old_leader.append_local(b"old leader stale scope widening".to_vec());
    let response = transport
        .append_entries(
            NodeId(4),
            append_request(&old_leader, vec![stale_entry], LogIndex(0), Term(0)),
        )
        .unwrap();
    assert!(!response.success);

    let new_leader = open_replication_db(new_leader_dir.path());
    let failover_evidence = checked_evidence(&new_leader, &[CellId(1), CellId(2)]);
    assert_same_pack_and_receipt(&committed_evidence, &failover_evidence);
}

#[test]
fn partition_heal_serves_only_committed_allowed_scope() {
    let leader_dir = tempfile::tempdir().unwrap();
    let majority_dir = tempfile::tempdir().unwrap();
    let mut leader_db = open_replication_db(leader_dir.path());
    seed_committed_cells(&mut leader_db);
    install_snapshot_over_peer(
        majority_dir.path(),
        leader_db.replication_snapshot_segment().unwrap(),
    );

    let voters = five_voters();
    let mut leader = ConsensusState::new(NodeId(1), voters.clone());
    let mut transport =
        transport_with_followers(&voters, &[NodeId(2), NodeId(3), NodeId(4), NodeId(5)]);
    transport.set_partitions(&[
        BTreeSet::from([NodeId(1), NodeId(2)]),
        BTreeSet::from([NodeId(3), NodeId(4), NodeId(5)]),
    ]);

    leader_db
        .put_cell(
            CellId(60),
            ready_project("minority", "minority write before partition heal"),
        )
        .unwrap();
    leader_db
        .put_cell(
            CellId(61),
            private_ready("minority", "minority private write before partition heal"),
        )
        .unwrap();
    let entry = leader.append_local(b"minority write before partition heal".to_vec());
    let minority_acks = transport
        .replicate_to_best_effort(
            [NodeId(2), NodeId(3), NodeId(4), NodeId(5)],
            append_request(&leader, vec![entry.clone()], LogIndex(0), Term(0)),
        )
        .unwrap();
    assert!(!leader.record_acks(entry.index, minority_acks).committed);

    let majority_during_partition = open_replication_db(majority_dir.path());
    checked_evidence(&majority_during_partition, &[CellId(1), CellId(2)]);
    drop(majority_during_partition);

    transport.heal_partitions();
    let healed_acks = transport
        .replicate_to_best_effort(
            [NodeId(2), NodeId(3), NodeId(4), NodeId(5)],
            append_request(&leader, vec![entry], LogIndex(0), Term(0)),
        )
        .unwrap();
    assert!(leader.record_acks(LogIndex(1), healed_acks).committed);
    install_snapshot_over_peer(
        majority_dir.path(),
        leader_db.replication_snapshot_segment().unwrap(),
    );

    let majority_after_heal = open_replication_db(majority_dir.path());
    checked_evidence(&majority_after_heal, &[CellId(1), CellId(2), CellId(60)]);
}
