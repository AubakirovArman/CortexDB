use std::collections::BTreeSet;

use cortex_engine::{
    decode_joint_membership_entry, decode_membership_entry, joint_membership_entry,
    membership_entry, recover_membership_config, recover_voting_config, ConsensusState, LogIndex,
    MembershipConfig, NodeId, ReplicatedEntry, ReplicationLog, Term, VotingConfig,
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
