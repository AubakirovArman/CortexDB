use std::collections::BTreeSet;

use cortex_engine::{
    ConsensusState, JointMembershipConfig, LogIndex, MembershipConfig, NodeId,
    ReplicationFollowerProgress, ReplicationFollowerProgressStore, VotingConfig,
};

#[test]
fn progress_store_reconcile_voters_removes_retired_and_seeds_joined() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("repair-progress.crp");
    let mut store = ReplicationFollowerProgressStore::open(&path).unwrap();
    store
        .record_many([
            ReplicationFollowerProgress::new(NodeId(2), LogIndex(4), LogIndex(5)),
            ReplicationFollowerProgress::new(NodeId(4), LogIndex(2), LogIndex(2)),
        ])
        .unwrap();

    store
        .reconcile_voters(
            NodeId(1),
            &BTreeSet::from([NodeId(1), NodeId(2), NodeId(3)]),
        )
        .unwrap();

    let reopened = ReplicationFollowerProgressStore::open(&path).unwrap();
    assert_eq!(reopened.progress().len(), 2);
    assert_eq!(
        reopened.progress().get(&NodeId(2)).copied(),
        Some(ReplicationFollowerProgress::new(
            NodeId(2),
            LogIndex(4),
            LogIndex(5)
        ))
    );
    assert_eq!(
        reopened.progress().get(&NodeId(3)).copied(),
        Some(ReplicationFollowerProgress::new(
            NodeId(3),
            LogIndex(0),
            LogIndex(0)
        ))
    );
    assert!(!reopened.progress().contains_key(&NodeId(4)));
}

#[test]
fn progress_store_reconcile_joint_config_tracks_union_then_stable_voters() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("repair-progress.crp");
    let mut store = ReplicationFollowerProgressStore::open(&path).unwrap();
    store
        .record(ReplicationFollowerProgress::new(
            NodeId(2),
            LogIndex(3),
            LogIndex(3),
        ))
        .unwrap();

    let joint = VotingConfig::Joint(
        JointMembershipConfig::new(
            BTreeSet::from([NodeId(1), NodeId(2)]),
            BTreeSet::from([NodeId(1), NodeId(3)]),
        )
        .unwrap(),
    );
    store.reconcile_voting_config(NodeId(1), &joint).unwrap();

    assert!(store.progress().contains_key(&NodeId(2)));
    assert_eq!(
        store.progress().get(&NodeId(3)).copied(),
        Some(ReplicationFollowerProgress::new(
            NodeId(3),
            LogIndex(0),
            LogIndex(0)
        ))
    );

    let stable = VotingConfig::Stable(
        MembershipConfig::new(BTreeSet::from([NodeId(1), NodeId(3)])).unwrap(),
    );
    store.reconcile_voting_config(NodeId(1), &stable).unwrap();

    assert!(!store.progress().contains_key(&NodeId(2)));
    assert!(store.progress().contains_key(&NodeId(3)));
}

#[test]
fn progress_store_reconcile_consensus_state_uses_current_leader_voters() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("repair-progress.crp");
    let mut store = ReplicationFollowerProgressStore::open(&path).unwrap();
    store
        .record(ReplicationFollowerProgress::new(
            NodeId(9),
            LogIndex(1),
            LogIndex(1),
        ))
        .unwrap();
    let leader = ConsensusState::new(NodeId(1), BTreeSet::from([NodeId(1), NodeId(2)]));

    store.reconcile_consensus_state(&leader).unwrap();

    assert_eq!(
        store.progress().get(&NodeId(2)).copied(),
        Some(ReplicationFollowerProgress::new(
            NodeId(2),
            LogIndex(0),
            LogIndex(0)
        ))
    );
    assert!(!store.progress().contains_key(&NodeId(9)));
}

#[test]
fn progress_store_reconcile_requires_local_member() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("repair-progress.crp");
    let mut store = ReplicationFollowerProgressStore::open(path).unwrap();

    let result = store.reconcile_voters(NodeId(1), &BTreeSet::from([NodeId(2), NodeId(3)]));

    assert!(result.is_err());
    assert!(store.progress().is_empty());
}

#[test]
fn progress_store_default_path_is_scoped_by_local_node() {
    let path = ReplicationFollowerProgressStore::default_path("/tmp/cortexdb", NodeId(7));

    assert!(path.ends_with("replication/node-7.repair-progress"));
}
