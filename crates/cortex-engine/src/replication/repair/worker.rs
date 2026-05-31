use std::collections::BTreeMap;

use crate::distributed::NodeId;
use crate::error::{EngineError, EngineResult};
use crate::replication::{
    run_replication_repair_cycle, send_replication_snapshot_request, ConsensusState,
    ReplicationFollowerProgress, ReplicationRecoveryPolicy, ReplicationRepairCycleResult,
    ReplicationSnapshotRepairRequest, ReplicationSnapshotSendPolicy, ReplicationSnapshotSendResult,
    ReplicationSnapshotTransport, ReplicationTransport, SnapshotSegment,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReplicationRepairWorkerPolicy {
    pub recovery: ReplicationRecoveryPolicy,
    pub snapshot: ReplicationSnapshotSendPolicy,
    pub max_ticks: usize,
}

impl Default for ReplicationRepairWorkerPolicy {
    fn default() -> Self {
        Self {
            recovery: ReplicationRecoveryPolicy::default(),
            snapshot: ReplicationSnapshotSendPolicy::default(),
            max_ticks: 8,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReplicationRepairWorkerTick {
    pub cycle: ReplicationRepairCycleResult,
    pub snapshots_sent: Vec<ReplicationSnapshotSendResult>,
    pub pending_snapshot_requests: Vec<ReplicationSnapshotRepairRequest>,
}

impl ReplicationRepairWorkerTick {
    pub fn made_progress(&self) -> bool {
        self.cycle.repaired_count() > 0 || !self.snapshots_sent.is_empty()
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ReplicationRepairWorkerReport {
    pub ticks: Vec<ReplicationRepairWorkerTick>,
}

impl ReplicationRepairWorkerReport {
    pub fn append_repairs(&self) -> usize {
        self.ticks
            .iter()
            .map(|tick| tick.cycle.repaired_count())
            .sum()
    }

    pub fn snapshots_sent(&self) -> usize {
        self.ticks
            .iter()
            .map(|tick| tick.snapshots_sent.len())
            .sum()
    }

    pub fn pending_snapshots(&self) -> usize {
        self.ticks
            .last()
            .map(|tick| tick.pending_snapshot_requests.len())
            .unwrap_or(0)
    }

    pub fn is_idle(&self) -> bool {
        self.ticks
            .last()
            .map(|tick| {
                !tick.made_progress()
                    && tick.cycle.snapshot_required_count() == 0
                    && tick.pending_snapshot_requests.is_empty()
            })
            .unwrap_or(true)
    }
}

pub trait ReplicationRepairProgressSource<T> {
    fn follower_progress(
        &mut self,
        leader: &ConsensusState,
        transport: &T,
    ) -> EngineResult<BTreeMap<NodeId, ReplicationFollowerProgress>>;
}

impl<T, F> ReplicationRepairProgressSource<T> for F
where
    F: FnMut(&ConsensusState, &T) -> EngineResult<BTreeMap<NodeId, ReplicationFollowerProgress>>,
{
    fn follower_progress(
        &mut self,
        leader: &ConsensusState,
        transport: &T,
    ) -> EngineResult<BTreeMap<NodeId, ReplicationFollowerProgress>> {
        self(leader, transport)
    }
}

pub trait ReplicationRepairSnapshotSource {
    fn snapshot_for_repair(
        &mut self,
        request: &ReplicationSnapshotRepairRequest,
    ) -> EngineResult<Option<SnapshotSegment>>;
}

impl<F> ReplicationRepairSnapshotSource for F
where
    F: FnMut(&ReplicationSnapshotRepairRequest) -> EngineResult<Option<SnapshotSegment>>,
{
    fn snapshot_for_repair(
        &mut self,
        request: &ReplicationSnapshotRepairRequest,
    ) -> EngineResult<Option<SnapshotSegment>> {
        self(request)
    }
}

pub fn run_replication_repair_worker<T, P, S>(
    leader: &ConsensusState,
    transport: &mut T,
    progress_source: &mut P,
    snapshot_source: &mut S,
    policy: ReplicationRepairWorkerPolicy,
) -> EngineResult<ReplicationRepairWorkerReport>
where
    T: ReplicationTransport + ReplicationSnapshotTransport,
    P: ReplicationRepairProgressSource<T>,
    S: ReplicationRepairSnapshotSource,
{
    if policy.max_ticks == 0 {
        return Err(EngineError::InvalidOperation);
    }

    let mut ticks = Vec::new();
    for _ in 0..policy.max_ticks {
        let progress = progress_source.follower_progress(leader, transport)?;
        let cycle = run_replication_repair_cycle(leader, transport, &progress, policy.recovery)?;
        let mut snapshots_sent = Vec::new();
        let mut pending_snapshot_requests = Vec::new();

        for request in &cycle.snapshot_requests {
            match snapshot_source.snapshot_for_repair(request)? {
                Some(snapshot) => snapshots_sent.push(send_replication_snapshot_request(
                    transport,
                    leader.current_term,
                    leader.local_node,
                    request,
                    &snapshot,
                    policy.snapshot,
                )?),
                None => pending_snapshot_requests.push(request.clone()),
            }
        }

        let tick = ReplicationRepairWorkerTick {
            cycle,
            snapshots_sent,
            pending_snapshot_requests,
        };
        let made_progress = tick.made_progress();
        let has_pending_snapshots = !tick.pending_snapshot_requests.is_empty();
        ticks.push(tick);

        if !made_progress || has_pending_snapshots {
            break;
        }
    }

    Ok(ReplicationRepairWorkerReport { ticks })
}
