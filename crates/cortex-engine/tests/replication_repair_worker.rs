use std::collections::{BTreeMap, BTreeSet};

use cortex_core::CommitSeq;
use cortex_engine::{
    run_replication_repair_worker, AppendEntriesRequest, AppendEntriesResponse, ConsensusState,
    ElectionState, InMemoryReplicationTransport, LogIndex, NodeId, ReplicatedEntry,
    ReplicationFollowerProgress, ReplicationRecoveryPolicy, ReplicationRepairWorkerPolicy,
    ReplicationSnapshotRepairRequest, ReplicationSnapshotSendPolicy, ReplicationSnapshotTransport,
    ReplicationTransport, SnapshotChunk, SnapshotSegment, Term, VoteRequest, VoteResponse,
};

#[test]
fn repair_worker_runs_append_and_snapshot_until_idle() {
    let voters = BTreeSet::from([NodeId(1), NodeId(2), NodeId(3)]);
    let leader = committed_leader(&voters, 5);
    let entries = leader.entries().to_vec();
    let mut transport = WorkerTransport::with_followers(&voters);
    seed_follower(
        &mut transport,
        &leader,
        NodeId(2),
        &entries[..3],
        LogIndex(3),
    );

    let mut progress_source = follower_progress;
    let mut snapshot_source = |request: &ReplicationSnapshotRepairRequest| {
        Ok(Some(SnapshotSegment {
            checkpoint_seq: CommitSeq(request.checkpoint.0),
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
                snapshot_threshold: 3,
            },
            snapshot: ReplicationSnapshotSendPolicy { max_chunk_bytes: 8 },
            max_ticks: 3,
        },
    )
    .unwrap();

    assert_eq!(report.ticks.len(), 2);
    assert_eq!(report.append_repairs(), 1);
    assert_eq!(report.snapshots_sent(), 1);
    assert_eq!(report.pending_snapshots(), 0);
    assert!(report.is_idle());
    assert_eq!(transport.peer_commit(NodeId(2)), Some(LogIndex(5)));
    assert_eq!(transport.peer_commit(NodeId(3)), Some(LogIndex(5)));
}

#[test]
fn repair_worker_returns_pending_snapshot_without_spinning() {
    let voters = BTreeSet::from([NodeId(1), NodeId(2)]);
    let leader = committed_leader(&voters, 5);
    let mut transport = WorkerTransport::with_followers(&voters);
    let mut progress_source = follower_progress;
    let mut snapshot_source = |_request: &ReplicationSnapshotRepairRequest| Ok(None);

    let report = run_replication_repair_worker(
        &leader,
        &mut transport,
        &mut progress_source,
        &mut snapshot_source,
        ReplicationRepairWorkerPolicy {
            recovery: ReplicationRecoveryPolicy {
                snapshot_threshold: 3,
            },
            snapshot: ReplicationSnapshotSendPolicy::default(),
            max_ticks: 3,
        },
    )
    .unwrap();

    assert_eq!(report.ticks.len(), 1);
    assert_eq!(report.append_repairs(), 0);
    assert_eq!(report.snapshots_sent(), 0);
    assert_eq!(report.pending_snapshots(), 1);
    assert!(!report.is_idle());
}

struct WorkerTransport {
    inner: InMemoryReplicationTransport,
    snapshot_bytes: BTreeMap<NodeId, u64>,
    snapshot_commits: BTreeMap<NodeId, LogIndex>,
}

impl WorkerTransport {
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
    transport: &mut WorkerTransport,
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
    transport: &WorkerTransport,
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
