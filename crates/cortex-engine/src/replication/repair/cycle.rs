use std::collections::BTreeMap;

use crate::distributed::NodeId;
use crate::error::{EngineError, EngineResult};

use super::{
    entries_after, plan_replication_repair_sweep, previous_log_term, ReplicationFollowerProgress,
    ReplicationRepairDecisionKind, ReplicationRepairResult, ReplicationRepairSchedule,
};
use crate::replication::{
    AppendEntriesRequest, ConsensusState, LogIndex, ReplicationRecoveryAction,
    ReplicationRecoveryPolicy, ReplicationTransport,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReplicationSnapshotRepairRequest {
    pub target: NodeId,
    pub checkpoint: LogIndex,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ReplicationRepairCycleResult {
    pub results: Vec<ReplicationRepairResult>,
    pub snapshot_requests: Vec<ReplicationSnapshotRepairRequest>,
}

impl ReplicationRepairCycleResult {
    pub fn repaired_count(&self) -> usize {
        self.results
            .iter()
            .filter(|result| result.append_sent && result.success)
            .count()
    }

    pub fn already_caught_up_count(&self) -> usize {
        self.results
            .iter()
            .filter(|result| result.success && !result.append_sent)
            .count()
    }

    pub fn snapshot_required_count(&self) -> usize {
        self.snapshot_requests.len()
    }
}

pub fn run_replication_repair_cycle<T: ReplicationTransport>(
    leader: &ConsensusState,
    transport: &mut T,
    follower_progress: &BTreeMap<NodeId, ReplicationFollowerProgress>,
    policy: ReplicationRecoveryPolicy,
) -> EngineResult<ReplicationRepairCycleResult> {
    let schedule = plan_replication_repair_sweep(leader, follower_progress, policy)?;
    execute_replication_repair_schedule(leader, transport, &schedule)
}

pub fn execute_replication_repair_schedule<T: ReplicationTransport>(
    leader: &ConsensusState,
    transport: &mut T,
    schedule: &ReplicationRepairSchedule,
) -> EngineResult<ReplicationRepairCycleResult> {
    let mut results = Vec::new();
    let mut snapshot_requests = Vec::new();
    for decision in &schedule.decisions {
        if !leader.voters.contains(&decision.target) || decision.target == leader.local_node {
            return Err(EngineError::InvalidOperation);
        }
        ensure_decision_matches_plan(decision.kind, decision.plan.action)?;
        match decision.plan.action {
            ReplicationRecoveryAction::AlreadyCaughtUp => results.push(ReplicationRepairResult {
                target: decision.target,
                plan: decision.plan,
                append_sent: false,
                success: true,
            }),
            ReplicationRecoveryAction::InstallSnapshot { checkpoint } => {
                snapshot_requests.push(ReplicationSnapshotRepairRequest {
                    target: decision.target,
                    checkpoint,
                });
            }
            ReplicationRecoveryAction::AppendEntries {
                from_exclusive,
                to_inclusive,
            } => {
                let entries = entries_after(leader.entries(), from_exclusive, to_inclusive)?;
                let prev_log_term = previous_log_term(leader.entries(), from_exclusive)?;
                let response = transport.append_entries(
                    decision.target,
                    AppendEntriesRequest {
                        term: leader.current_term,
                        leader_id: leader.local_node,
                        prev_log_index: from_exclusive,
                        prev_log_term,
                        entries,
                        leader_commit: leader.commit_index,
                    },
                )?;
                results.push(ReplicationRepairResult {
                    target: decision.target,
                    plan: decision.plan,
                    append_sent: true,
                    success: response.success,
                });
            }
        }
    }
    Ok(ReplicationRepairCycleResult {
        results,
        snapshot_requests,
    })
}

fn ensure_decision_matches_plan(
    kind: ReplicationRepairDecisionKind,
    action: ReplicationRecoveryAction,
) -> EngineResult<()> {
    let matches = matches!(
        (kind, action),
        (
            ReplicationRepairDecisionKind::AlreadyCaughtUp,
            ReplicationRecoveryAction::AlreadyCaughtUp
        ) | (
            ReplicationRepairDecisionKind::AppendEntries,
            ReplicationRecoveryAction::AppendEntries { .. }
        ) | (
            ReplicationRepairDecisionKind::InstallSnapshot,
            ReplicationRecoveryAction::InstallSnapshot { .. }
        )
    );
    if matches {
        Ok(())
    } else {
        Err(EngineError::InvalidOperation)
    }
}
