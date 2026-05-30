use std::collections::BTreeSet;

use cortex_engine::{
    decode_membership_entry, membership_entry, recover_membership_config, LogIndex,
    MembershipConfig, NodeId, ReplicatedEntry, ReplicationLog, Term,
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
}
