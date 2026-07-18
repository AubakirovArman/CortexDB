#![cfg(feature = "experimental-replication")]

#[path = "multi_agent_cluster_consistency/support.rs"]
mod support;

use std::collections::BTreeSet;

use cortex_core::{CellId, CommitSeq};
use cortex_engine::{
    classify_memory_visibility, ConsensusState, LogIndex, MemoryConsistencyLevel, NodeId, Term,
};

use support::{
    append_request, assert_visible, commit_shared, elect_new_leader_after_failover, five_voters,
    handoff_request, install_snapshot_over_peer, open_cluster_db, transport_with_followers, view,
    SHARED_SCOPE,
};

#[test]
fn read_your_writes_survives_follower_read_and_leader_failover() {
    let leader_dir = tempfile::tempdir().unwrap();
    let follower_dir = tempfile::tempdir().unwrap();
    let post_failover_follower_dir = tempfile::tempdir().unwrap();
    let mut leader = open_cluster_db(leader_dir.path());
    let writer = view(1, &[SHARED_SCOPE], &[SHARED_SCOPE]);

    let first_seq = commit_shared(&mut leader, &writer, CellId(10), "cluster alpha");
    let visibility = classify_memory_visibility(&writer, writer.agent_id, SHARED_SCOPE, first_seq);
    assert_eq!(visibility.level, MemoryConsistencyLevel::SharedImmediate);
    assert_eq!(visibility.visible_after_seq, first_seq);

    install_snapshot_over_peer(
        follower_dir.path(),
        leader.replication_snapshot_segment().unwrap(),
    );
    let follower = open_cluster_db(follower_dir.path());
    assert!(follower.current_seq() >= first_seq);
    assert_visible(&follower, &writer, &[CellId(10)]);
    drop(follower);

    elect_new_leader_after_failover();
    let mut promoted = open_cluster_db(follower_dir.path());
    let second_seq = commit_shared(&mut promoted, &writer, CellId(11), "cluster beta");
    assert!(second_seq > first_seq);
    assert_visible(&promoted, &writer, &[CellId(10), CellId(11)]);

    install_snapshot_over_peer(
        post_failover_follower_dir.path(),
        promoted.replication_snapshot_segment().unwrap(),
    );
    let post_failover_follower = open_cluster_db(post_failover_follower_dir.path());
    assert!(post_failover_follower.current_seq() >= second_seq);
    assert_visible(&post_failover_follower, &writer, &[CellId(10), CellId(11)]);
}

#[test]
fn monotonic_read_and_handoff_survive_partition_heal() {
    let leader_dir = tempfile::tempdir().unwrap();
    let follower_dir = tempfile::tempdir().unwrap();
    let mut leader_db = open_cluster_db(leader_dir.path());
    let writer = view(1, &[SHARED_SCOPE], &[SHARED_SCOPE]);
    let reader = view(2, &[SHARED_SCOPE], &[]);

    let first_seq = commit_shared(&mut leader_db, &writer, CellId(20), "cluster gamma");
    install_snapshot_over_peer(
        follower_dir.path(),
        leader_db.replication_snapshot_segment().unwrap(),
    );
    let stale_follower = open_cluster_db(follower_dir.path());
    assert_visible(&stale_follower, &reader, &[CellId(20)]);
    let last_seen_seq = stale_follower.current_seq();
    assert!(last_seen_seq >= first_seq);

    let future_seq = CommitSeq(last_seen_seq.0 + 1);
    let future_handoff = stale_follower.plan_agent_handoff(
        &writer,
        &reader,
        handoff_request(first_seq, future_seq, last_seen_seq, "future"),
    );
    assert!(future_handoff.is_err());
    drop(stale_follower);

    let voters = five_voters();
    let mut consensus = ConsensusState::new(NodeId(1), voters.clone());
    let mut transport =
        transport_with_followers(&voters, &[NodeId(2), NodeId(3), NodeId(4), NodeId(5)]);
    transport.set_partitions(&[
        BTreeSet::from([NodeId(1), NodeId(2)]),
        BTreeSet::from([NodeId(3), NodeId(4), NodeId(5)]),
    ]);
    let entry = consensus.append_local(b"cluster delta".to_vec());
    let minority_acks = transport
        .replicate_to_best_effort(
            [NodeId(2), NodeId(3), NodeId(4), NodeId(5)],
            append_request(&consensus, vec![entry.clone()], LogIndex(0), Term(0)),
        )
        .unwrap();
    assert!(!consensus.record_acks(entry.index, minority_acks).committed);

    transport.heal_partitions();
    let healed_acks = transport
        .replicate_to_best_effort(
            [NodeId(2), NodeId(3), NodeId(4), NodeId(5)],
            append_request(&consensus, vec![entry], LogIndex(0), Term(0)),
        )
        .unwrap();
    assert!(consensus.record_acks(LogIndex(1), healed_acks).committed);

    let second_seq = commit_shared(&mut leader_db, &writer, CellId(21), "cluster delta");
    assert!(second_seq > last_seen_seq);
    install_snapshot_over_peer(
        follower_dir.path(),
        leader_db.replication_snapshot_segment().unwrap(),
    );
    let healed_follower = open_cluster_db(follower_dir.path());
    assert!(healed_follower.current_seq() >= second_seq);
    assert_visible(&healed_follower, &reader, &[CellId(20), CellId(21)]);

    let visibility = classify_memory_visibility(&reader, writer.agent_id, SHARED_SCOPE, second_seq);
    assert_eq!(visibility.level, MemoryConsistencyLevel::SharedImmediate);
    assert_eq!(visibility.visible_after_seq, second_seq);
    let handoff = healed_follower
        .plan_agent_handoff(
            &writer,
            &reader,
            handoff_request(first_seq, second_seq, last_seen_seq, "healed"),
        )
        .unwrap();
    assert_eq!(handoff.level, MemoryConsistencyLevel::SharedSequenced);
    assert_eq!(handoff.visible_after_seq, second_seq);
    assert_eq!(handoff.required_after_seq, last_seen_seq);
}
