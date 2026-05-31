use std::collections::BTreeSet;

use crate::distributed::NodeId;
use crate::error::{EngineError, EngineResult};

use super::consensus::{ConsensusState, LogIndex, ReplicatedEntry, Term};
use super::membership::{
    decode_joint_membership_entry, joint_membership_entry, membership_entry,
    JointConsensusDecision, JointMembershipConfig,
};
use super::transport::{AppendEntriesRequest, ReplicationTransport};
use super::ReplicationLog;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MembershipRotationPhase {
    JointNotCommitted,
    StableNotCommitted,
    Complete,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MembershipRotationResult {
    pub phase: MembershipRotationPhase,
    pub joint_entry: ReplicatedEntry,
    pub stable_entry: Option<ReplicatedEntry>,
    pub joint_decision: JointConsensusDecision,
    pub stable_decision: Option<JointConsensusDecision>,
    pub final_voters: BTreeSet<NodeId>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MembershipRotationResumeResult {
    pub phase: MembershipRotationPhase,
    pub stable_entry: ReplicatedEntry,
    pub stable_decision: JointConsensusDecision,
    pub final_voters: BTreeSet<NodeId>,
}

pub fn rotate_membership_with_joint_consensus<T: ReplicationTransport>(
    leader: &mut ConsensusState,
    transport: &mut T,
    replication_log: &ReplicationLog,
    new_voters: BTreeSet<NodeId>,
) -> EngineResult<MembershipRotationResult> {
    validate_rotation_inputs(leader, &new_voters)?;
    let old_voters = leader.voters.clone();
    let joint_config = JointMembershipConfig::new(old_voters.clone(), new_voters.clone())?;
    catch_up_voters(leader, transport, joint_config.voters_union())?;
    let joint_entry = append_membership_entry(
        leader,
        replication_log,
        joint_membership_entry(
            leader.current_term,
            next_log_index(leader)?,
            old_voters,
            new_voters.clone(),
        )?
        .payload,
    )?;
    let joint_acks = replicate_entry(leader, transport, joint_config.voters_union(), &joint_entry)?;
    let joint_decision =
        leader.record_joint_consensus_acks(joint_entry.index, &joint_config, joint_acks);
    if !joint_decision.committed {
        return Ok(MembershipRotationResult {
            phase: MembershipRotationPhase::JointNotCommitted,
            joint_entry,
            stable_entry: None,
            joint_decision,
            stable_decision: None,
            final_voters: leader.voters.clone(),
        });
    }

    let stable_entry = append_membership_entry(
        leader,
        replication_log,
        membership_entry(
            leader.current_term,
            next_log_index(leader)?,
            new_voters.clone(),
        )?
        .payload,
    )?;
    let stable_acks = replicate_entry(
        leader,
        transport,
        joint_config.voters_union(),
        &stable_entry,
    )?;
    let stable_decision =
        leader.record_joint_consensus_acks(stable_entry.index, &joint_config, stable_acks);
    if !stable_decision.committed {
        return Ok(MembershipRotationResult {
            phase: MembershipRotationPhase::StableNotCommitted,
            joint_entry,
            stable_entry: Some(stable_entry),
            joint_decision,
            stable_decision: Some(stable_decision),
            final_voters: leader.voters.clone(),
        });
    }

    leader.reconfigure_voters(new_voters.clone())?;
    Ok(MembershipRotationResult {
        phase: MembershipRotationPhase::Complete,
        joint_entry,
        stable_entry: Some(stable_entry),
        joint_decision,
        stable_decision: Some(stable_decision),
        final_voters: new_voters,
    })
}

pub fn resume_joint_membership_rotation<T: ReplicationTransport>(
    leader: &mut ConsensusState,
    transport: &mut T,
    replication_log: &ReplicationLog,
    joint_config: JointMembershipConfig,
) -> EngineResult<MembershipRotationResumeResult> {
    validate_committed_joint_entry(leader, &joint_config)?;
    validate_rotation_inputs(leader, &joint_config.new_voters)?;
    catch_up_voters(leader, transport, joint_config.voters_union())?;
    let stable_entry = append_membership_entry(
        leader,
        replication_log,
        membership_entry(
            leader.current_term,
            next_log_index(leader)?,
            joint_config.new_voters.clone(),
        )?
        .payload,
    )?;
    let stable_acks = replicate_entry(
        leader,
        transport,
        joint_config.voters_union(),
        &stable_entry,
    )?;
    let stable_decision =
        leader.record_joint_consensus_acks(stable_entry.index, &joint_config, stable_acks);
    if !stable_decision.committed {
        return Ok(MembershipRotationResumeResult {
            phase: MembershipRotationPhase::StableNotCommitted,
            stable_entry,
            stable_decision,
            final_voters: leader.voters.clone(),
        });
    }

    leader.reconfigure_voters(joint_config.new_voters.clone())?;
    Ok(MembershipRotationResumeResult {
        phase: MembershipRotationPhase::Complete,
        stable_entry,
        stable_decision,
        final_voters: joint_config.new_voters,
    })
}

fn validate_rotation_inputs(
    leader: &ConsensusState,
    new_voters: &BTreeSet<NodeId>,
) -> EngineResult<()> {
    if new_voters.is_empty() || !new_voters.contains(&leader.local_node) {
        return Err(EngineError::InvalidOperation);
    }
    Ok(())
}

fn append_membership_entry(
    leader: &mut ConsensusState,
    replication_log: &ReplicationLog,
    payload: Vec<u8>,
) -> EngineResult<ReplicatedEntry> {
    let entry = leader.append_local(payload);
    replication_log.append(&entry)?;
    Ok(entry)
}

fn validate_committed_joint_entry(
    leader: &ConsensusState,
    joint_config: &JointMembershipConfig,
) -> EngineResult<()> {
    let latest_joint = leader
        .entries()
        .iter()
        .filter(|entry| entry.index <= leader.commit_index)
        .filter_map(|entry| match decode_joint_membership_entry(entry) {
            Ok(Some(config)) => Some(Ok(config)),
            Ok(None) => None,
            Err(error) => Some(Err(error)),
        })
        .next_back()
        .transpose()?;
    if latest_joint.as_ref() == Some(joint_config) {
        Ok(())
    } else {
        Err(EngineError::InvalidOperation)
    }
}

fn replicate_entry<T: ReplicationTransport>(
    leader: &ConsensusState,
    transport: &mut T,
    voters: BTreeSet<NodeId>,
    entry: &ReplicatedEntry,
) -> EngineResult<BTreeSet<NodeId>> {
    let (prev_log_index, prev_log_term) = previous_log_position(leader, entry.index)?;
    let request = AppendEntriesRequest {
        term: leader.current_term,
        leader_id: leader.local_node,
        prev_log_index,
        prev_log_term,
        entries: vec![entry.clone()],
        leader_commit: leader.commit_index,
    };
    let mut acks = BTreeSet::from([leader.local_node]);
    for voter in voters {
        if voter == leader.local_node {
            continue;
        }
        if let Ok(response) = transport.append_entries(voter, request.clone()) {
            if response.success {
                acks.insert(response.follower);
            }
        }
    }
    Ok(acks)
}

fn catch_up_voters<T: ReplicationTransport>(
    leader: &ConsensusState,
    transport: &mut T,
    voters: BTreeSet<NodeId>,
) -> EngineResult<()> {
    if voters.is_empty() || leader.entries().is_empty() {
        return Ok(());
    }
    let request = AppendEntriesRequest {
        term: leader.current_term,
        leader_id: leader.local_node,
        prev_log_index: LogIndex(0),
        prev_log_term: Term(0),
        entries: leader.entries().to_vec(),
        leader_commit: leader.commit_index,
    };
    for voter in voters {
        if voter == leader.local_node {
            continue;
        }
        let _ = transport.append_entries(voter, request.clone());
    }
    Ok(())
}

fn previous_log_position(
    leader: &ConsensusState,
    entry_index: LogIndex,
) -> EngineResult<(LogIndex, Term)> {
    let previous = entry_index
        .0
        .checked_sub(1)
        .ok_or(EngineError::InvalidOperation)?;
    if previous == 0 {
        return Ok((LogIndex(0), Term(0)));
    }
    leader
        .entries()
        .iter()
        .find(|entry| entry.index == LogIndex(previous))
        .map(|entry| (entry.index, entry.term))
        .ok_or(EngineError::InvalidOperation)
}

fn next_log_index(leader: &ConsensusState) -> EngineResult<LogIndex> {
    leader
        .last_log_index()
        .0
        .checked_add(1)
        .map(LogIndex)
        .ok_or(EngineError::InvalidOperation)
}
