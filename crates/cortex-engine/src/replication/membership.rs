use std::collections::BTreeSet;

use crate::distributed::NodeId;
use crate::error::{EngineError, EngineResult};

use super::{LogIndex, ReplicatedEntry, Term};

const MEMBERSHIP_PAYLOAD_PREFIX: &[u8] = b"CORTEXDB_MEMBERSHIP_V1 ";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MembershipConfig {
    pub voters: BTreeSet<NodeId>,
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

pub fn membership_entry(
    term: Term,
    index: LogIndex,
    voters: BTreeSet<NodeId>,
) -> EngineResult<ReplicatedEntry> {
    Ok(MembershipConfig::new(voters)?.to_entry(term, index))
}

pub fn decode_membership_entry(entry: &ReplicatedEntry) -> EngineResult<Option<MembershipConfig>> {
    if !entry.payload.starts_with(MEMBERSHIP_PAYLOAD_PREFIX) {
        return Ok(None);
    }
    let body = &entry.payload[MEMBERSHIP_PAYLOAD_PREFIX.len()..];
    let text = std::str::from_utf8(body).map_err(|_| EngineError::InvalidOperation)?;
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
    MembershipConfig::new(voters).map(Some)
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

fn encode_membership_payload(voters: &BTreeSet<NodeId>) -> Vec<u8> {
    let mut out = MEMBERSHIP_PAYLOAD_PREFIX.to_vec();
    for (index, voter) in voters.iter().enumerate() {
        if index > 0 {
            out.push(b',');
        }
        out.extend_from_slice(voter.0.to_string().as_bytes());
    }
    out
}
