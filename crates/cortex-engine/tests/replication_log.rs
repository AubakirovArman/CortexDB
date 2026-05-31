use std::collections::BTreeSet;

use cortex_engine::{
    ConsensusState, EngineError, LogIndex, NodeId, ReplicatedEntry, ReplicationLog, Term,
};

#[test]
fn replication_log_persists_entries_through_aclog() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("replication.aclog");
    let entry = ReplicatedEntry {
        term: Term(3),
        index: LogIndex(7),
        payload: b"put cell".to_vec(),
    };

    let log = ReplicationLog::open(&path).unwrap();
    let ack = log.append(&entry).unwrap();
    assert!(ack.durable_lsn > 0);
    assert_eq!(log.path(), path.as_path());
    log.close().unwrap();

    assert_eq!(ReplicationLog::recover_entries(&path).unwrap(), vec![entry]);
}

#[test]
fn replication_log_recovers_consensus_state() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("replication.aclog");
    let voters = BTreeSet::from([NodeId(1), NodeId(2), NodeId(3)]);
    let mut consensus = ConsensusState::new(NodeId(1), voters.clone());
    let entry = consensus.append_local(b"replicate this".to_vec());
    let decision = consensus.record_acks(entry.index, BTreeSet::from([NodeId(1), NodeId(2)]));
    assert!(decision.committed);

    let log = ReplicationLog::open(&path).unwrap();
    log.append(&entry).unwrap();
    log.close().unwrap();

    let recovered =
        ReplicationLog::recover_consensus(&path, NodeId(1), voters, consensus.commit_index)
            .unwrap();
    assert_eq!(recovered.current_term, Term(1));
    assert_eq!(recovered.committed_entries(), vec![entry]);
}

#[test]
fn missing_replication_log_recovers_empty_entries() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("missing.aclog");

    assert!(ReplicationLog::recover_entries(path).unwrap().is_empty());
}

#[test]
fn recover_consensus_rejects_commit_index_beyond_log() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("replication.aclog");

    let result = ReplicationLog::recover_consensus(
        &path,
        NodeId(1),
        BTreeSet::from([NodeId(1), NodeId(2), NodeId(3)]),
        LogIndex(1),
    );

    assert!(matches!(result, Err(EngineError::InvalidOperation)));
}

#[test]
fn recover_consensus_rejects_non_contiguous_indexes() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("replication.aclog");
    let log = ReplicationLog::open(&path).unwrap();
    log.append(&ReplicatedEntry {
        term: Term(1),
        index: LogIndex(1),
        payload: b"first".to_vec(),
    })
    .unwrap();
    log.append(&ReplicatedEntry {
        term: Term(1),
        index: LogIndex(3),
        payload: b"gap".to_vec(),
    })
    .unwrap();
    log.close().unwrap();

    let result = ReplicationLog::recover_consensus(
        &path,
        NodeId(1),
        BTreeSet::from([NodeId(1), NodeId(2), NodeId(3)]),
        LogIndex(1),
    );

    assert!(matches!(result, Err(EngineError::InvalidOperation)));
}
