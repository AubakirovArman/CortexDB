use std::collections::{BTreeMap, BTreeSet};
use std::sync::mpsc;
use std::time::Duration;

use cortex_core::CommitSeq;
use cortex_engine::{
    spawn_replication_repair_background_task, AppendEntriesRequest, AppendEntriesResponse,
    ConsensusState, ElectionState, InMemoryReplicationTransport, LogIndex, NodeId, ReplicatedEntry,
    ReplicationFollowerProgress, ReplicationRecoveryPolicy, ReplicationRepairBackgroundPolicy,
    ReplicationRepairWorkerPolicy, ReplicationSnapshotRepairRequest, ReplicationSnapshotSendPolicy,
    ReplicationSnapshotTransport, ReplicationTransport, SnapshotChunk, SnapshotSegment, Term,
    VoteRequest, VoteResponse,
};

#[test]
fn background_repair_task_runs_worker_until_idle() {
    let voters = BTreeSet::from([NodeId(1), NodeId(2), NodeId(3)]);
    let leader = committed_leader(&voters, 5);
    let entries = leader.entries().to_vec();
    let mut transport = BackgroundTransport::with_followers(&voters);
    seed_follower(
        &mut transport,
        &leader,
        NodeId(2),
        &entries[..3],
        LogIndex(3),
    );

    let handle = spawn_replication_repair_background_task(
        leader,
        transport,
        follower_progress,
        snapshot_for_request,
        ReplicationRepairBackgroundPolicy {
            worker: ReplicationRepairWorkerPolicy {
                recovery: ReplicationRecoveryPolicy {
                    snapshot_threshold: 3,
                },
                snapshot: ReplicationSnapshotSendPolicy { max_chunk_bytes: 8 },
                max_ticks: 4,
            },
            tick_interval: Duration::from_millis(1),
            max_runs: None,
            stop_when_idle: true,
        },
    )
    .unwrap();

    let report = handle.join().unwrap();

    assert_eq!(report.runs.len(), 1);
    assert_eq!(report.append_repairs(), 1);
    assert_eq!(report.snapshots_sent(), 1);
    assert_eq!(report.pending_snapshots(), 0);
    assert!(report.last_run_idle());
    assert!(!report.stopped);
}

#[test]
fn background_repair_task_stops_between_ticks() {
    let voters = BTreeSet::from([NodeId(1), NodeId(2)]);
    let leader = committed_leader(&voters, 1);
    let transport = BackgroundTransport::with_followers(&voters);
    let (observed_tx, observed_rx) = mpsc::channel();
    let mut observed_tx = Some(observed_tx);
    let progress = move |leader: &ConsensusState,
                         transport: &BackgroundTransport|
          -> cortex_engine::EngineResult<
        BTreeMap<NodeId, ReplicationFollowerProgress>,
    > {
        if let Some(sender) = observed_tx.take() {
            let _ = sender.send(());
        }
        follower_progress(leader, transport)
    };

    let handle = spawn_replication_repair_background_task(
        leader,
        transport,
        progress,
        snapshot_for_request,
        ReplicationRepairBackgroundPolicy {
            worker: ReplicationRepairWorkerPolicy::default(),
            tick_interval: Duration::from_secs(60),
            max_runs: None,
            stop_when_idle: false,
        },
    )
    .unwrap();
    observed_rx.recv_timeout(Duration::from_secs(2)).unwrap();

    let report = handle.stop().unwrap();

    assert!(report.stopped);
    assert_eq!(report.runs.len(), 1);
    assert!(report.last_run_idle());
}

struct BackgroundTransport {
    inner: InMemoryReplicationTransport,
    snapshot_bytes: BTreeMap<NodeId, u64>,
    snapshot_commits: BTreeMap<NodeId, LogIndex>,
}

impl BackgroundTransport {
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

impl ReplicationTransport for BackgroundTransport {
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

impl ReplicationSnapshotTransport for BackgroundTransport {
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

fn committed_leader(voters: &BTreeSet<NodeId>, entries: usize) -> ConsensusState {
    let mut leader = ConsensusState::new(NodeId(1), voters.clone());
    for index in 0..entries {
        let entry = leader.append_local(format!("entry {index}").into_bytes());
        leader.record_acks(entry.index, BTreeSet::from([NodeId(1), NodeId(2)]));
    }
    leader
}

fn seed_follower(
    transport: &mut BackgroundTransport,
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

fn follower_progress(
    leader: &ConsensusState,
    transport: &BackgroundTransport,
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

fn snapshot_for_request(
    request: &ReplicationSnapshotRepairRequest,
) -> cortex_engine::EngineResult<Option<SnapshotSegment>> {
    Ok(Some(SnapshotSegment {
        checkpoint_seq: CommitSeq(request.checkpoint.0),
        cells: Vec::new(),
    }))
}
