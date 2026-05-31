use std::collections::BTreeSet;
use std::sync::{Arc, RwLock};

use cortex_core::CommitSeq;
use cortex_engine::{
    run_replication_repair_worker, send_replication_snapshot_request, AppendEntriesRequest,
    AppendEntriesResponse, ConsensusState, ElectionState, EngineError,
    InMemoryReplicationTransport, LogIndex, NodeId, ReplicatedEntry, ReplicationFollowerProgress,
    ReplicationFollowerProgressStore, ReplicationProgressRecordingTransport,
    ReplicationRecoveryPolicy, ReplicationRepairWorkerPolicy, ReplicationSnapshotRepairRequest,
    ReplicationSnapshotSendPolicy, ReplicationSnapshotTransport, ReplicationStoredProgressSource,
    ReplicationTransport, SnapshotChunk, SnapshotSegment, Term, VoteRequest, VoteResponse,
};

#[test]
fn progress_store_persists_records_across_reopen() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("repair-progress.crp");
    let mut store = ReplicationFollowerProgressStore::open(&path).unwrap();

    store
        .record_many([
            ReplicationFollowerProgress::new(NodeId(2), LogIndex(3), LogIndex(4)),
            ReplicationFollowerProgress::new(NodeId(3), LogIndex(1), LogIndex(1)),
        ])
        .unwrap();

    let reopened = ReplicationFollowerProgressStore::open(&path).unwrap();
    assert_eq!(reopened.progress().len(), 2);
    assert_eq!(
        reopened.progress().get(&NodeId(2)).copied(),
        Some(ReplicationFollowerProgress::new(
            NodeId(2),
            LogIndex(3),
            LogIndex(4)
        ))
    );
    assert_eq!(reopened.path(), path.as_path());
}

#[test]
fn progress_store_rejects_observed_index_before_commit_index() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("repair-progress.crp");
    let mut store = ReplicationFollowerProgressStore::open(&path).unwrap();

    let result = store.record(ReplicationFollowerProgress::new(
        NodeId(2),
        LogIndex(5),
        LogIndex(4),
    ));

    assert!(matches!(result, Err(EngineError::InvalidOperation)));
    assert!(store.progress().is_empty());
}

#[test]
fn stored_progress_source_feeds_repair_worker() {
    let voters = BTreeSet::from([NodeId(1), NodeId(2), NodeId(3)]);
    let leader = committed_leader(&voters, 2);
    let mut transport = WorkerTransport::with_followers(&voters);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("repair-progress.crp");
    let mut store = ReplicationFollowerProgressStore::open(path).unwrap();
    store
        .record_many([
            ReplicationFollowerProgress::new(NodeId(2), LogIndex(0), LogIndex(0)),
            ReplicationFollowerProgress::new(NodeId(3), leader.commit_index, leader.commit_index),
        ])
        .unwrap();
    let store = Arc::new(RwLock::new(store));
    let mut progress_source = ReplicationStoredProgressSource::new(store);
    let mut snapshot_source = |_request: &ReplicationSnapshotRepairRequest| {
        Ok(Some(SnapshotSegment {
            checkpoint_seq: CommitSeq(0),
            cells: Vec::new(),
        }))
    };

    let report = run_replication_repair_worker(
        &leader,
        &mut transport,
        &mut progress_source,
        &mut snapshot_source,
        ReplicationRepairWorkerPolicy {
            recovery: ReplicationRecoveryPolicy {
                snapshot_threshold: 10,
            },
            snapshot: ReplicationSnapshotSendPolicy::default(),
            max_ticks: 1,
        },
    )
    .unwrap();

    assert_eq!(report.append_repairs(), 1);
    assert_eq!(transport.peer_commit(NodeId(2)), Some(LogIndex(2)));
}

#[test]
fn recording_transport_persists_successful_append_entries_ack() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("repair-progress.crp");
    let store = Arc::new(RwLock::new(
        ReplicationFollowerProgressStore::open(&path).unwrap(),
    ));
    let voters = BTreeSet::from([NodeId(1), NodeId(2)]);
    let inner = WorkerTransport::with_followers(&voters);
    let mut transport = ReplicationProgressRecordingTransport::new(inner, store);
    let entry = ReplicatedEntry {
        term: Term(1),
        index: LogIndex(1),
        payload: b"entry".to_vec(),
    };

    let response = transport
        .append_entries(
            NodeId(2),
            AppendEntriesRequest {
                term: Term(1),
                leader_id: NodeId(1),
                prev_log_index: LogIndex(0),
                prev_log_term: Term(0),
                entries: vec![entry],
                leader_commit: LogIndex(1),
            },
        )
        .unwrap();

    assert!(response.success);
    let reopened = ReplicationFollowerProgressStore::open(&path).unwrap();
    assert_eq!(
        reopened.progress().get(&NodeId(2)).copied(),
        Some(ReplicationFollowerProgress::new(
            NodeId(2),
            LogIndex(1),
            LogIndex(1)
        ))
    );
}

#[test]
fn recording_transport_skips_failed_append_entries_ack() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("repair-progress.crp");
    let store = Arc::new(RwLock::new(
        ReplicationFollowerProgressStore::open(&path).unwrap(),
    ));
    let voters = BTreeSet::from([NodeId(1), NodeId(2)]);
    let inner = WorkerTransport::with_followers(&voters);
    let mut transport = ReplicationProgressRecordingTransport::new(inner, store);

    let response = transport
        .append_entries(
            NodeId(2),
            AppendEntriesRequest {
                term: Term(1),
                leader_id: NodeId(1),
                prev_log_index: LogIndex(9),
                prev_log_term: Term(1),
                entries: Vec::new(),
                leader_commit: LogIndex(1),
            },
        )
        .unwrap();

    assert!(!response.success);
    let reopened = ReplicationFollowerProgressStore::open(&path).unwrap();
    assert!(reopened.progress().is_empty());
}

#[test]
fn recording_transport_persists_final_snapshot_ack() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("repair-progress.crp");
    let store = Arc::new(RwLock::new(
        ReplicationFollowerProgressStore::open(&path).unwrap(),
    ));
    let inner = RecordingSnapshotTransport::default();
    let mut transport = ReplicationProgressRecordingTransport::new(inner, store);
    let snapshot = SnapshotSegment {
        checkpoint_seq: CommitSeq(4),
        cells: Vec::new(),
    };

    send_replication_snapshot_request(
        &mut transport,
        Term(2),
        NodeId(1),
        &ReplicationSnapshotRepairRequest {
            target: NodeId(2),
            checkpoint: LogIndex(4),
        },
        &snapshot,
        ReplicationSnapshotSendPolicy { max_chunk_bytes: 3 },
    )
    .unwrap();

    let reopened = ReplicationFollowerProgressStore::open(&path).unwrap();
    assert_eq!(
        reopened.progress().get(&NodeId(2)).copied(),
        Some(ReplicationFollowerProgress::new(
            NodeId(2),
            LogIndex(4),
            LogIndex(4)
        ))
    );
}

struct WorkerTransport {
    inner: InMemoryReplicationTransport,
}

impl WorkerTransport {
    fn with_followers(voters: &BTreeSet<NodeId>) -> Self {
        let mut inner = InMemoryReplicationTransport::default();
        for node in voters.iter().copied().filter(|node| *node != NodeId(1)) {
            let mut state = ElectionState::new(node, voters.clone());
            state.current_term = Term(1);
            inner.register_peer(state);
        }
        Self { inner }
    }

    fn peer_commit(&self, node: NodeId) -> Option<LogIndex> {
        self.inner.peer_commit(node)
    }
}

impl ReplicationTransport for WorkerTransport {
    fn request_vote(
        &mut self,
        target: NodeId,
        request: VoteRequest,
    ) -> cortex_engine::EngineResult<VoteResponse> {
        self.inner.request_vote(target, request)
    }

    fn append_entries(
        &mut self,
        target: NodeId,
        request: AppendEntriesRequest,
    ) -> cortex_engine::EngineResult<AppendEntriesResponse> {
        self.inner.append_entries(target, request)
    }
}

impl ReplicationSnapshotTransport for WorkerTransport {
    fn send_snapshot_chunk(
        &mut self,
        _target: NodeId,
        _chunk: &SnapshotChunk,
    ) -> cortex_engine::EngineResult<u64> {
        Err(EngineError::InvalidOperation)
    }
}

#[derive(Default)]
struct RecordingSnapshotTransport {
    received: u64,
}

impl ReplicationSnapshotTransport for RecordingSnapshotTransport {
    fn send_snapshot_chunk(
        &mut self,
        _target: NodeId,
        chunk: &SnapshotChunk,
    ) -> cortex_engine::EngineResult<u64> {
        self.received += chunk.payload.len() as u64;
        Ok(self.received)
    }
}

fn committed_leader(voters: &BTreeSet<NodeId>, entries: usize) -> ConsensusState {
    let mut leader = ConsensusState::new(NodeId(1), voters.clone());
    for index in 0..entries {
        let entry = leader.append_local(format!("entry {index}").into_bytes());
        leader.record_acks(entry.index, BTreeSet::from([NodeId(1), NodeId(2)]));
    }
    leader
}
