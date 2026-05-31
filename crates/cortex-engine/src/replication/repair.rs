use crate::distributed::NodeId;
use crate::error::{EngineError, EngineResult};

use super::consensus::{ConsensusState, LogIndex, ReplicatedEntry, Term};
use super::recovery::{
    plan_replication_recovery, ReplicationRecoveryAction, ReplicationRecoveryPlan,
    ReplicationRecoveryPolicy,
};
use super::transport::{AppendEntriesRequest, ReplicationTransport};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReplicationRepairResult {
    pub target: NodeId,
    pub plan: ReplicationRecoveryPlan,
    pub append_sent: bool,
    pub success: bool,
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
