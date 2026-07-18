#![cfg(feature = "experimental-replication")]

use std::collections::BTreeSet;
use std::sync::{Arc, RwLock};
use std::time::Duration;

use cortex_engine::{
    spawn_replication_repair_background_task_with_progress_store, AppendEntriesRequest,
    AppendEntriesResponse, ConsensusState, ElectionState, EngineError,
    InMemoryReplicationTransport, LogIndex, NodeId, ReplicationFollowerProgress,
    ReplicationFollowerProgressStore, ReplicationRecoveryPolicy, ReplicationRepairBackgroundPolicy,
    ReplicationRepairWorkerPolicy, ReplicationSnapshotRepairRequest, ReplicationSnapshotSendPolicy,
    ReplicationSnapshotTransport, ReplicationTransport, SnapshotChunk, SnapshotSegment, Term,
    VoteRequest, VoteResponse,
};

#[test]
fn progress_store_background_runtime_reads_and_records_from_one_store() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("repair-progress.crp");
    let mut store = ReplicationFollowerProgressStore::open(&path).unwrap();
    store
        .record(ReplicationFollowerProgress::new(
            NodeId(2),
            LogIndex(0),
            LogIndex(0),
        ))
        .unwrap();
    let store = Arc::new(RwLock::new(store));
    let voters = BTreeSet::from([NodeId(1), NodeId(2)]);
    let leader = committed_leader(&voters, 2);
    let transport = RuntimeTransport::with_followers(&voters);

    let handle = spawn_replication_repair_background_task_with_progress_store(
        leader,
        transport,
        store,
        no_snapshot_request,
        ReplicationRepairBackgroundPolicy {
            worker: ReplicationRepairWorkerPolicy {
                recovery: ReplicationRecoveryPolicy {
                    snapshot_threshold: 10,
                },
                snapshot: ReplicationSnapshotSendPolicy::default(),
                max_ticks: 4,
            },
            tick_interval: Duration::from_millis(1),
            max_runs: None,
            stop_when_idle: true,
        },
    )
    .unwrap();

    let report = handle.join().unwrap();
    let reopened = ReplicationFollowerProgressStore::open(&path).unwrap();

    assert_eq!(report.append_repairs(), 1);
    assert!(report.last_run_idle());
    assert_eq!(
        reopened.progress().get(&NodeId(2)).copied(),
        Some(ReplicationFollowerProgress::new(
            NodeId(2),
            LogIndex(2),
            LogIndex(2)
        ))
    );
}

struct RuntimeTransport {
    inner: InMemoryReplicationTransport,
}

impl RuntimeTransport {
    fn with_followers(voters: &BTreeSet<NodeId>) -> Self {
        let mut inner = InMemoryReplicationTransport::default();
        for node in voters.iter().copied().filter(|node| *node != NodeId(1)) {
            let mut state = ElectionState::new(node, voters.clone());
            state.current_term = Term(1);
            inner.register_peer(state);
        }
        Self { inner }
    }
}

impl ReplicationTransport for RuntimeTransport {
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

impl ReplicationSnapshotTransport for RuntimeTransport {
    fn send_snapshot_chunk(
        &mut self,
        _target: NodeId,
        _chunk: &SnapshotChunk,
    ) -> cortex_engine::EngineResult<u64> {
        Err(EngineError::InvalidOperation)
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

fn no_snapshot_request(
    _request: &ReplicationSnapshotRepairRequest,
) -> cortex_engine::EngineResult<Option<SnapshotSegment>> {
    Ok(None)
}
