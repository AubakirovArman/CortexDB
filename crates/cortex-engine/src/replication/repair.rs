use std::collections::BTreeMap;

use crate::distributed::NodeId;
use crate::error::{EngineError, EngineResult};

use super::consensus::{ConsensusState, LogIndex, ReplicatedEntry, Term};
use super::recovery::{
    plan_replication_recovery, ReplicationRecoveryAction, ReplicationRecoveryPlan,
    ReplicationRecoveryPolicy,
};
use super::transport::{AppendEntriesRequest, ReplicationTransport};

mod cycle;
mod snapshot_sender;

pub use cycle::{
    execute_replication_repair_schedule, run_replication_repair_cycle,
    ReplicationRepairCycleResult, ReplicationSnapshotRepairRequest,
};
pub use snapshot_sender::{
    send_replication_snapshot_request, ReplicationSnapshotSendPolicy,
    ReplicationSnapshotSendResult, ReplicationSnapshotTransport,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReplicationRepairResult {
    pub target: NodeId,
    pub plan: ReplicationRecoveryPlan,
    pub append_sent: bool,
    pub success: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReplicationFollowerProgress {
    pub node: NodeId,
    pub commit_index: LogIndex,
    pub last_observed_index: LogIndex,
}

impl ReplicationFollowerProgress {
    pub fn new(node: NodeId, commit_index: LogIndex, last_observed_index: LogIndex) -> Self {
        Self {
            node,
            commit_index,
            last_observed_index,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReplicationRepairDecisionKind {
    AlreadyCaughtUp,
    AppendEntries,
    InstallSnapshot,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReplicationRepairDecision {
    pub target: NodeId,
    pub plan: ReplicationRecoveryPlan,
    pub kind: ReplicationRepairDecisionKind,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ReplicationRepairSchedule {
    pub decisions: Vec<ReplicationRepairDecision>,
}

impl ReplicationRepairSchedule {
    pub fn append_entries_count(&self) -> usize {
        self.decisions
            .iter()
            .filter(|decision| decision.kind == ReplicationRepairDecisionKind::AppendEntries)
            .count()
    }

    pub fn snapshot_required_count(&self) -> usize {
        self.decisions
            .iter()
            .filter(|decision| decision.kind == ReplicationRepairDecisionKind::InstallSnapshot)
            .count()
    }

    pub fn already_caught_up_count(&self) -> usize {
        self.decisions
            .iter()
            .filter(|decision| decision.kind == ReplicationRepairDecisionKind::AlreadyCaughtUp)
            .count()
    }

    pub fn is_idle(&self) -> bool {
        self.append_entries_count() == 0 && self.snapshot_required_count() == 0
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ReplicationRepairSweepResult {
    pub results: Vec<ReplicationRepairResult>,
}

impl ReplicationRepairSweepResult {
    pub fn repaired_count(&self) -> usize {
        self.results
            .iter()
            .filter(|result| result.append_sent && result.success)
            .count()
    }

    pub fn snapshot_required_count(&self) -> usize {
        self.results
            .iter()
            .filter(|result| {
                matches!(
                    result.plan.action,
                    ReplicationRecoveryAction::InstallSnapshot { .. }
                )
            })
            .count()
    }

    pub fn already_caught_up_count(&self) -> usize {
        self.results
            .iter()
            .filter(|result| {
                matches!(
                    result.plan.action,
                    ReplicationRecoveryAction::AlreadyCaughtUp
                )
            })
            .count()
    }
}

pub fn plan_replication_repair_sweep(
    leader: &ConsensusState,
    follower_progress: &BTreeMap<NodeId, ReplicationFollowerProgress>,
    policy: ReplicationRecoveryPolicy,
) -> EngineResult<ReplicationRepairSchedule> {
    let mut decisions = Vec::new();
    for target in leader
        .voters
        .iter()
        .copied()
        .filter(|node| *node != leader.local_node)
    {
        let progress = follower_progress
            .get(&target)
            .copied()
            .unwrap_or_else(|| ReplicationFollowerProgress::new(target, LogIndex(0), LogIndex(0)));
        validate_progress(target, progress, leader.commit_index)?;
        let plan = plan_replication_recovery(progress.commit_index, leader.commit_index, policy);
        decisions.push(ReplicationRepairDecision {
            target,
            plan,
            kind: repair_decision_kind(plan.action),
        });
    }
    Ok(ReplicationRepairSchedule { decisions })
}

pub fn repair_lagging_voters<T: ReplicationTransport>(
    leader: &ConsensusState,
    transport: &mut T,
    follower_commits: &BTreeMap<NodeId, LogIndex>,
    policy: ReplicationRecoveryPolicy,
) -> EngineResult<ReplicationRepairSweepResult> {
    let mut results = Vec::new();
    for target in leader
        .voters
        .iter()
        .copied()
        .filter(|node| *node != leader.local_node)
    {
        let follower_commit = follower_commits.get(&target).copied().unwrap_or_default();
        results.push(repair_lagging_voter(
            leader,
            transport,
            target,
            follower_commit,
            policy,
        )?);
    }
    Ok(ReplicationRepairSweepResult { results })
}

pub fn repair_lagging_voter<T: ReplicationTransport>(
    leader: &ConsensusState,
    transport: &mut T,
    target: NodeId,
    follower_commit: LogIndex,
    policy: ReplicationRecoveryPolicy,
) -> EngineResult<ReplicationRepairResult> {
    if !leader.voters.contains(&target) {
        return Err(EngineError::InvalidOperation);
    }
    let plan = plan_replication_recovery(follower_commit, leader.commit_index, policy);
    match plan.action {
        ReplicationRecoveryAction::AlreadyCaughtUp => Ok(ReplicationRepairResult {
            target,
            plan,
            append_sent: false,
            success: true,
        }),
        ReplicationRecoveryAction::InstallSnapshot { .. } => Ok(ReplicationRepairResult {
            target,
            plan,
            append_sent: false,
            success: false,
        }),
        ReplicationRecoveryAction::AppendEntries {
            from_exclusive,
            to_inclusive,
        } => {
            let entries = entries_after(leader.entries(), from_exclusive, to_inclusive)?;
            let prev_log_term = previous_log_term(leader.entries(), from_exclusive)?;
            let response = transport.append_entries(
                target,
                AppendEntriesRequest {
                    term: leader.current_term,
                    leader_id: leader.local_node,
                    prev_log_index: from_exclusive,
                    prev_log_term,
                    entries,
                    leader_commit: leader.commit_index,
                },
            )?;
            Ok(ReplicationRepairResult {
                target,
                plan,
                append_sent: true,
                success: response.success,
            })
        }
    }
}

fn entries_after(
    entries: &[ReplicatedEntry],
    from_exclusive: LogIndex,
    to_inclusive: LogIndex,
) -> EngineResult<Vec<ReplicatedEntry>> {
    let selected = entries
        .iter()
        .filter(|entry| entry.index > from_exclusive && entry.index <= to_inclusive)
        .cloned()
        .collect::<Vec<_>>();
    if selected.len() as u64 == to_inclusive.0.saturating_sub(from_exclusive.0) {
        Ok(selected)
    } else {
        Err(EngineError::InvalidOperation)
    }
}

fn previous_log_term(entries: &[ReplicatedEntry], index: LogIndex) -> EngineResult<Term> {
    if index == LogIndex(0) {
        return Ok(Term(0));
    }
    entries
        .iter()
        .find(|entry| entry.index == index)
        .map(|entry| entry.term)
        .ok_or(EngineError::InvalidOperation)
}

fn validate_progress(
    target: NodeId,
    progress: ReplicationFollowerProgress,
    leader_commit: LogIndex,
) -> EngineResult<()> {
    if progress.node != target
        || progress.commit_index > leader_commit
        || progress.last_observed_index < progress.commit_index
    {
        return Err(EngineError::InvalidOperation);
    }
    Ok(())
}

fn repair_decision_kind(action: ReplicationRecoveryAction) -> ReplicationRepairDecisionKind {
    match action {
        ReplicationRecoveryAction::AlreadyCaughtUp => {
            ReplicationRepairDecisionKind::AlreadyCaughtUp
        }
        ReplicationRecoveryAction::AppendEntries { .. } => {
            ReplicationRepairDecisionKind::AppendEntries
        }
        ReplicationRecoveryAction::InstallSnapshot { .. } => {
            ReplicationRepairDecisionKind::InstallSnapshot
        }
    }
}
