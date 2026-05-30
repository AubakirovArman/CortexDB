use std::collections::{BTreeMap, BTreeSet};

use crate::distributed::NodeId;
use crate::error::{EngineError, EngineResult};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Term(pub u64);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LogIndex(pub u64);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReplicatedEntry {
    pub term: Term,
    pub index: LogIndex,
    pub payload: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommitDecision {
    pub index: LogIndex,
    pub committed: bool,
    pub acknowledgements: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConsensusState {
    pub local_node: NodeId,
    pub voters: BTreeSet<NodeId>,
    pub current_term: Term,
    pub commit_index: LogIndex,
    log: Vec<ReplicatedEntry>,
}

impl ConsensusState {
    pub fn new(local_node: NodeId, voters: BTreeSet<NodeId>) -> Self {
        Self {
            local_node,
            voters,
            current_term: Term(1),
            commit_index: LogIndex(0),
            log: Vec::new(),
        }
    }

    pub fn append_local(&mut self, payload: Vec<u8>) -> ReplicatedEntry {
        let entry = ReplicatedEntry {
            term: self.current_term,
            index: LogIndex(self.log.len() as u64 + 1),
            payload,
        };
        self.log.push(entry.clone());
        entry
    }

    pub fn record_acks(&mut self, index: LogIndex, acks: BTreeSet<NodeId>) -> CommitDecision {
        let valid_acks = acks
            .intersection(&self.voters)
            .copied()
            .collect::<BTreeSet<_>>();
        let committed = valid_acks.len() >= self.majority() && self.is_current_term_entry(index);
        if committed && index > self.commit_index {
            self.commit_index = index;
        }
        CommitDecision {
            index,
            committed,
            acknowledgements: valid_acks.len(),
        }
    }

    pub fn record_match_indexes(
        &mut self,
        mut match_indexes: BTreeMap<NodeId, LogIndex>,
    ) -> CommitDecision {
        match_indexes.insert(self.local_node, self.last_log_index());
        for entry in self.log.iter().rev() {
            if entry.index <= self.commit_index || entry.term != self.current_term {
                continue;
            }
            let acknowledgements = self
                .voters
                .iter()
                .filter(|node| match_indexes.get(node).copied().unwrap_or_default() >= entry.index)
                .count();
            if acknowledgements >= self.majority() {
                self.commit_index = entry.index;
                return CommitDecision {
                    index: entry.index,
                    committed: true,
                    acknowledgements,
                };
            }
        }
        CommitDecision {
            index: self.commit_index,
            committed: false,
            acknowledgements: 0,
        }
    }

    pub fn committed_entries(&self) -> Vec<ReplicatedEntry> {
        self.log
            .iter()
            .filter(|entry| entry.index <= self.commit_index)
            .cloned()
            .collect()
    }

    pub fn entries(&self) -> &[ReplicatedEntry] {
        &self.log
    }

    pub fn last_log_index(&self) -> LogIndex {
        self.log.last().map(|entry| entry.index).unwrap_or_default()
    }

    pub fn last_log_term(&self) -> Term {
        self.log.last().map(|entry| entry.term).unwrap_or_default()
    }

    pub fn recover(
        local_node: NodeId,
        voters: BTreeSet<NodeId>,
        entries: Vec<ReplicatedEntry>,
        commit_index: LogIndex,
    ) -> Self {
        let current_term = entries
            .iter()
            .map(|entry| entry.term)
            .max()
            .unwrap_or(Term(1));
        Self {
            local_node,
            voters,
            current_term,
            commit_index,
            log: entries,
        }
    }

    pub fn reconfigure_voters(&mut self, voters: BTreeSet<NodeId>) -> EngineResult<()> {
        validate_voter_set(self.local_node, &voters)?;
        self.voters = voters;
        Ok(())
    }

    fn majority(&self) -> usize {
        (self.voters.len() / 2) + 1
    }

    fn is_current_term_entry(&self, index: LogIndex) -> bool {
        self.log
            .iter()
            .any(|entry| entry.index == index && entry.term == self.current_term)
    }
}

pub(crate) fn validate_voter_set(
    local_node: NodeId,
    voters: &BTreeSet<NodeId>,
) -> EngineResult<()> {
    if voters.is_empty() || !voters.contains(&local_node) {
        return Err(EngineError::InvalidOperation);
    }
    Ok(())
}
