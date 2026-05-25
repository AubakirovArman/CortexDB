use std::collections::BTreeSet;

use crate::distributed::NodeId;

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
        let committed = valid_acks.len() >= self.majority();
        if committed && index > self.commit_index {
            self.commit_index = index;
        }
        CommitDecision {
            index,
            committed,
            acknowledgements: valid_acks.len(),
        }
    }

    pub fn committed_entries(&self) -> Vec<ReplicatedEntry> {
        self.log
            .iter()
            .filter(|entry| entry.index <= self.commit_index)
            .cloned()
            .collect()
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

    fn majority(&self) -> usize {
        (self.voters.len() / 2) + 1
    }
}
