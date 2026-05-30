use std::collections::BTreeSet;

use crate::distributed::NodeId;
use crate::error::{EngineError, EngineResult};

use super::{LogIndex, ReplicatedEntry, Term};

const MEMBERSHIP_PAYLOAD_PREFIX: &[u8] = b"CORTEXDB_MEMBERSHIP_V1 ";
const JOINT_MEMBERSHIP_PAYLOAD_PREFIX: &[u8] = b"CORTEXDB_MEMBERSHIP_JOINT_V1 ";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MembershipConfig {
    pub voters: BTreeSet<NodeId>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JointMembershipConfig {
    pub old_voters: BTreeSet<NodeId>,
    pub new_voters: BTreeSet<NodeId>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VotingConfig {
    Stable(MembershipConfig),
    Joint(JointMembershipConfig),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JointConsensusDecision {
    pub index: LogIndex,
    pub committed: bool,
    pub old_acknowledgements: usize,
    pub new_acknowledgements: usize,
}

impl MembershipConfig {
    pub fn new(voters: BTreeSet<NodeId>) -> EngineResult<Self> {
        if voters.is_empty() {
            return Err(EngineError::InvalidOperation);
        }
        Ok(Self { voters })
    }

    pub fn to_entry(&self, term: Term, index: LogIndex) -> ReplicatedEntry {
        ReplicatedEntry {
            term,
            index,
            payload: encode_membership_payload(&self.voters),
        }
    }
}

impl JointMembershipConfig {
    pub fn new(old_voters: BTreeSet<NodeId>, new_voters: BTreeSet<NodeId>) -> EngineResult<Self> {
        if old_voters.is_empty() || new_voters.is_empty() {
            return Err(EngineError::InvalidOperation);
        }
        Ok(Self {
            old_voters,
            new_voters,
        })
    }

    pub fn to_entry(&self, term: Term, index: LogIndex) -> ReplicatedEntry {
        ReplicatedEntry {
            term,
            index,
            payload: encode_joint_membership_payload(&self.old_voters, &self.new_voters),
        }
    }

    pub fn voters_union(&self) -> BTreeSet<NodeId> {
        self.old_voters
            .union(&self.new_voters)
            .copied()
            .collect::<BTreeSet<_>>()
    }
}

pub fn membership_entry(
    term: Term,
    index: LogIndex,
    voters: BTreeSet<NodeId>,
) -> EngineResult<ReplicatedEntry> {
    Ok(MembershipConfig::new(voters)?.to_entry(term, index))
}

pub fn joint_membership_entry(
    term: Term,
    index: LogIndex,
    old_voters: BTreeSet<NodeId>,
    new_voters: BTreeSet<NodeId>,
) -> EngineResult<ReplicatedEntry> {
    Ok(JointMembershipConfig::new(old_voters, new_voters)?.to_entry(term, index))
}

pub fn decode_membership_entry(entry: &ReplicatedEntry) -> EngineResult<Option<MembershipConfig>> {
    if !entry.payload.starts_with(MEMBERSHIP_PAYLOAD_PREFIX) {
        return Ok(None);
    }
    decode_voter_set(&entry.payload[MEMBERSHIP_PAYLOAD_PREFIX.len()..])
        .and_then(MembershipConfig::new)
        .map(Some)
}

pub fn decode_joint_membership_entry(
    entry: &ReplicatedEntry,
) -> EngineResult<Option<JointMembershipConfig>> {
    if !entry.payload.starts_with(JOINT_MEMBERSHIP_PAYLOAD_PREFIX) {
        return Ok(None);
    }
    let body = &entry.payload[JOINT_MEMBERSHIP_PAYLOAD_PREFIX.len()..];
    let text = std::str::from_utf8(body).map_err(|_| EngineError::InvalidOperation)?;
    let (old, new) = text.split_once('|').ok_or(EngineError::InvalidOperation)?;
    JointMembershipConfig::new(
        decode_voter_set(old.as_bytes())?,
        decode_voter_set(new.as_bytes())?,
    )
    .map(Some)
}

pub fn recover_membership_config(
    entries: &[ReplicatedEntry],
    fallback_voters: BTreeSet<NodeId>,
    commit_index: LogIndex,
) -> EngineResult<MembershipConfig> {
    let mut config = MembershipConfig::new(fallback_voters)?;
    for entry in entries {
        if entry.index > commit_index {
            continue;
        }
        if let Some(next) = decode_membership_entry(entry)? {
            config = next;
        }
    }
    Ok(config)
}

pub fn recover_voting_config(
    entries: &[ReplicatedEntry],
    fallback_voters: BTreeSet<NodeId>,
    commit_index: LogIndex,
) -> EngineResult<VotingConfig> {
    let mut config = VotingConfig::Stable(MembershipConfig::new(fallback_voters)?);
    for entry in entries {
        if entry.index > commit_index {
            continue;
        }
        if let Some(next) = decode_membership_entry(entry)? {
            config = VotingConfig::Stable(next);
        } else if let Some(next) = decode_joint_membership_entry(entry)? {
            config = VotingConfig::Joint(next);
        }
    }
    Ok(config)
}

pub fn evaluate_joint_consensus(
    index: LogIndex,
    config: &JointMembershipConfig,
    acks: &BTreeSet<NodeId>,
) -> JointConsensusDecision {
    let old_acknowledgements = acknowledgement_count(&config.old_voters, acks);
    let new_acknowledgements = acknowledgement_count(&config.new_voters, acks);
    JointConsensusDecision {
        index,
        committed: old_acknowledgements >= majority(&config.old_voters)
            && new_acknowledgements >= majority(&config.new_voters),
        old_acknowledgements,
        new_acknowledgements,
    }
}

fn encode_membership_payload(voters: &BTreeSet<NodeId>) -> Vec<u8> {
    let mut out = MEMBERSHIP_PAYLOAD_PREFIX.to_vec();
    encode_voter_set(voters, &mut out);
    out
}

fn encode_joint_membership_payload(
    old_voters: &BTreeSet<NodeId>,
    new_voters: &BTreeSet<NodeId>,
) -> Vec<u8> {
    let mut out = JOINT_MEMBERSHIP_PAYLOAD_PREFIX.to_vec();
    encode_voter_set(old_voters, &mut out);
    out.push(b'|');
    encode_voter_set(new_voters, &mut out);
    out
}

fn encode_voter_set(voters: &BTreeSet<NodeId>, out: &mut Vec<u8>) {
    for (index, voter) in voters.iter().enumerate() {
        if index > 0 {
            out.push(b',');
        }
        out.extend_from_slice(voter.0.to_string().as_bytes());
    }
}

fn decode_voter_set(bytes: &[u8]) -> EngineResult<BTreeSet<NodeId>> {
    let text = std::str::from_utf8(bytes).map_err(|_| EngineError::InvalidOperation)?;
    let mut voters = BTreeSet::new();
    for token in text.split(',') {
        if token.is_empty() {
            return Err(EngineError::InvalidOperation);
        }
        let value = token
            .parse::<u64>()
            .map_err(|_| EngineError::InvalidOperation)?;
        voters.insert(NodeId(value));
    }
    Ok(voters)
}

fn acknowledgement_count(voters: &BTreeSet<NodeId>, acks: &BTreeSet<NodeId>) -> usize {
    voters.intersection(acks).count()
}

fn majority(voters: &BTreeSet<NodeId>) -> usize {
    (voters.len() / 2) + 1
}
