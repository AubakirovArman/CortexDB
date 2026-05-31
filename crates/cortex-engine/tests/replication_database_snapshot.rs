use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, RwLock};
use std::time::Duration;

use cortex_core::CellId;
use cortex_engine::{
    spawn_replication_repair_background_task, AppendEntriesRequest, AppendEntriesResponse,
    ConsensusState, Database, ElectionState, InMemoryReplicationTransport, LogIndex, NodeId,
    ReplicationDatabaseSnapshotSource, ReplicationFollowerProgress, ReplicationRecoveryPolicy,
    ReplicationRepairBackgroundPolicy, ReplicationRepairSnapshotSource,
    ReplicationRepairWorkerPolicy, ReplicationSnapshotSendPolicy, ReplicationSnapshotTransport,
    ReplicationTransport, SnapshotChunk, Term, VoteRequest, VoteResponse,
};

#[test]
fn database_snapshot_source_returns_current_storage_snapshot() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_cell(CellId(42), b"snapshot payload".to_vec())
        .unwrap();

    let snapshot = db.replication_snapshot_segment().unwrap();

    assert_eq!(snapshot.checkpoint_seq.0, 1);
    assert_eq!(snapshot.cells.len(), 1);
    assert_eq!(snapshot.cells[0].cell_id, 42);
    assert_eq!(snapshot.cells[0].payload, b"snapshot payload");

    let mut source = ReplicationDatabaseSnapshotSource::new(Arc::new(RwLock::new(db)));
    assert!(source
        .snapshot_for_repair(&cortex_engine::ReplicationSnapshotRepairRequest {
            target: NodeId(2),
            checkpoint: LogIndex(1),
        })
        .unwrap()
        .is_some());
    assert!(source
        .snapshot_for_repair(&cortex_engine::ReplicationSnapshotRepairRequest {
            target: NodeId(2),
            checkpoint: LogIndex(2),
        })
        .unwrap()
        .is_none());
}

#[test]
fn database_snapshot_source_feeds_background_repair_task() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_cell(CellId(7), b"repair snapshot payload".to_vec())
        .unwrap();
    let snapshot_source = ReplicationDatabaseSnapshotSource::new(Arc::new(RwLock::new(db)));

    let voters = BTreeSet::from([NodeId(1), NodeId(2)]);
    let mut leader = ConsensusState::new(NodeId(1), voters.clone());
    let entry = leader.append_local(b"entry 1".to_vec());
    leader.record_acks(entry.index, voters.clone());

    let handle = spawn_replication_repair_background_task(
        leader,
        SnapshotRecordingTransport::with_followers(&voters),
        follower_progress,
        snapshot_source,
        ReplicationRepairBackgroundPolicy {
            worker: ReplicationRepairWorkerPolicy {
                recovery: ReplicationRecoveryPolicy {
                    snapshot_threshold: 0,
                },
                snapshot: ReplicationSnapshotSendPolicy {
                    max_chunk_bytes: 16,
                },
                max_ticks: 3,
            },
            tick_interval: Duration::from_millis(1),
            max_runs: None,
            stop_when_idle: true,
        },
    )
    .unwrap();

    let report = handle.join().unwrap();

    assert_eq!(report.runs.len(), 1);
    assert_eq!(report.snapshots_sent(), 1);
    assert_eq!(report.pending_snapshots(), 0);
    assert!(report.last_run_idle());
}

struct SnapshotRecordingTransport {
    inner: InMemoryReplicationTransport,
    snapshot_bytes: BTreeMap<NodeId, u64>,
    snapshot_commits: BTreeMap<NodeId, LogIndex>,
}

impl SnapshotRecordingTransport {
    fn with_followers(voters: &BTreeSet<NodeId>) -> Self {
        let mut inner = InMemoryReplicationTransport::default();
        for node in voters.iter().copied().filter(|node| *node != NodeId(1)) {
            let mut state = ElectionState::new(node, voters.clone());
            state.current_term = Term(1);
            inner.register_peer(state);
        }
        Self {
            inner,
            snapshot_bytes: BTreeMap::new(),
            snapshot_commits: BTreeMap::new(),
        }
    }

    fn peer_commit(&self, node: NodeId) -> Option<LogIndex> {
        self.snapshot_commits
            .get(&node)
            .copied()
            .or_else(|| self.inner.peer_commit(node))
    }
}

impl ReplicationTransport for SnapshotRecordingTransport {
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

impl ReplicationSnapshotTransport for SnapshotRecordingTransport {
    fn send_snapshot_chunk(
        &mut self,
        target: NodeId,
        chunk: &SnapshotChunk,
    ) -> cortex_engine::EngineResult<u64> {
        let received = self.snapshot_bytes.entry(target).or_default();
        *received += chunk.payload.len() as u64;
        if chunk.last {
            self.snapshot_commits.insert(target, chunk.leader_commit);
        }
        Ok(*received)
    }
}

fn follower_progress(
    leader: &ConsensusState,
    transport: &SnapshotRecordingTransport,
) -> cortex_engine::EngineResult<BTreeMap<NodeId, ReplicationFollowerProgress>> {
    Ok(leader
        .voters
        .iter()
        .copied()
        .filter(|node| *node != leader.local_node)
        .map(|node| {
            let commit = transport.peer_commit(node).unwrap_or_default();
            (node, ReplicationFollowerProgress::new(node, commit, commit))
        })
        .collect())
}
