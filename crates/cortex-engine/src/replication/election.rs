use std::collections::BTreeSet;

use crate::distributed::NodeId;

use super::{LogIndex, Term};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ElectionRole {
    Follower,
    Candidate,
    Leader,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VoteRequest {
    pub term: Term,
    pub candidate_id: NodeId,
    pub last_log_index: LogIndex,
    pub last_log_term: Term,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VoteResponse {
    pub term: Term,
    pub voter: NodeId,
    pub vote_granted: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ElectionOutcome {
    pub role: ElectionRole,
    pub leader: Option<NodeId>,
    pub votes: usize,
    pub elected: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ElectionState {
    pub local_node: NodeId,
    pub voters: BTreeSet<NodeId>,
    pub current_term: Term,
    pub role: ElectionRole,
    pub voted_for: Option<NodeId>,
    pub leader: Option<NodeId>,
    votes_received: BTreeSet<NodeId>,
    last_log_index: LogIndex,
    last_log_term: Term,
}

impl ElectionState {
    pub fn new(local_node: NodeId, voters: BTreeSet<NodeId>) -> Self {
        Self {
            local_node,
            voters,
            current_term: Term(0),
            role: ElectionRole::Follower,
            voted_for: None,
            leader: None,
            votes_received: BTreeSet::new(),
            last_log_index: LogIndex(0),
            last_log_term: Term(0),
        }
    }

    pub fn set_last_log(&mut self, index: LogIndex, term: Term) {
        self.last_log_index = index;
        self.last_log_term = term;
    }

    pub fn start_election(&mut self) -> VoteRequest {
        self.current_term = Term(self.current_term.0 + 1);
        self.role = ElectionRole::Candidate;
        self.voted_for = Some(self.local_node);
        self.leader = None;
        self.votes_received = BTreeSet::from([self.local_node]);
        VoteRequest {
            term: self.current_term,
            candidate_id: self.local_node,
            last_log_index: self.last_log_index,
            last_log_term: self.last_log_term,
        }
    }

    pub fn handle_vote_request(&mut self, request: &VoteRequest) -> VoteResponse {
        if request.term < self.current_term {
            return self.vote_response(false);
        }
        if request.term > self.current_term {
            self.step_down(request.term);
        }
        let log_ok = request.last_log_term > self.last_log_term
            || (request.last_log_term == self.last_log_term
                && request.last_log_index >= self.last_log_index);
        let vote_ok = self.voted_for.is_none() || self.voted_for == Some(request.candidate_id);
        let granted = log_ok && vote_ok && self.voters.contains(&request.candidate_id);
        if granted {
            self.voted_for = Some(request.candidate_id);
            self.leader = None;
        }
        self.vote_response(granted)
    }

    pub fn record_vote(&mut self, response: VoteResponse) -> ElectionOutcome {
        if response.term > self.current_term {
            self.step_down(response.term);
            return self.outcome(false);
        }
        if self.role != ElectionRole::Candidate || response.term != self.current_term {
            return self.outcome(false);
        }
        if response.vote_granted && self.voters.contains(&response.voter) {
            self.votes_received.insert(response.voter);
        }
        let elected = self.votes_received.len() >= self.majority();
        if elected {
            self.role = ElectionRole::Leader;
            self.leader = Some(self.local_node);
        }
        self.outcome(elected)
    }

    pub fn accept_leader(&mut self, term: Term, leader: NodeId) -> bool {
        if term < self.current_term || !self.voters.contains(&leader) {
            return false;
        }
        if term == self.current_term && self.leader.is_some_and(|known| known != leader) {
            return false;
        }
        if term > self.current_term {
            self.step_down(term);
        }
        self.role = ElectionRole::Follower;
        self.leader = Some(leader);
        true
    }

    fn step_down(&mut self, term: Term) {
        self.current_term = term;
        self.role = ElectionRole::Follower;
        self.voted_for = None;
        self.leader = None;
        self.votes_received.clear();
    }

    fn vote_response(&self, granted: bool) -> VoteResponse {
        VoteResponse {
            term: self.current_term,
            voter: self.local_node,
            vote_granted: granted,
        }
    }

    fn outcome(&self, elected: bool) -> ElectionOutcome {
        ElectionOutcome {
            role: self.role,
            leader: self.leader,
            votes: self.votes_received.len(),
            elected,
        }
    }

    fn majority(&self) -> usize {
        (self.voters.len() / 2) + 1
    }
}
