use std::collections::{BTreeMap, BTreeSet};

use crate::distributed::NodeId;
use crate::error::{EngineError, EngineResult};

use super::election::{ElectionState, VoteRequest, VoteResponse};
use super::log_matching;
use super::{LogIndex, ReplicatedEntry, Term};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AppendEntriesRequest {
    pub term: Term,
    pub leader_id: NodeId,
    pub prev_log_index: LogIndex,
    pub prev_log_term: Term,
    pub entries: Vec<ReplicatedEntry>,
    pub leader_commit: LogIndex,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AppendEntriesResponse {
    pub term: Term,
    pub follower: NodeId,
    pub success: bool,
    pub match_index: LogIndex,
}

pub trait ReplicationTransport {
    fn request_vote(&mut self, target: NodeId, request: VoteRequest) -> EngineResult<VoteResponse>;

    fn append_entries(
        &mut self,
        target: NodeId,
        request: AppendEntriesRequest,
    ) -> EngineResult<AppendEntriesResponse>;
}

#[derive(Clone, Debug, Default)]
pub struct InMemoryReplicationTransport {
    peers: BTreeMap<NodeId, ElectionState>,
    logs: BTreeMap<NodeId, Vec<ReplicatedEntry>>,
    commits: BTreeMap<NodeId, LogIndex>,
    allowed_links: Option<BTreeSet<(NodeId, NodeId)>>,
}

impl InMemoryReplicationTransport {
    pub fn register_peer(&mut self, state: ElectionState) {
        self.logs.entry(state.local_node).or_default();
        self.commits.entry(state.local_node).or_default();
        self.peers.insert(state.local_node, state);
    }

    pub fn peer_log(&self, node: NodeId) -> Option<&[ReplicatedEntry]> {
        self.logs.get(&node).map(Vec::as_slice)
    }

    pub fn peer_commit(&self, node: NodeId) -> Option<LogIndex> {
        self.commits.get(&node).copied()
    }

    pub fn set_partitions(&mut self, groups: &[BTreeSet<NodeId>]) {
        let mut allowed_links = BTreeSet::new();
        for group in groups {
            for from in group {
                for to in group {
                    if from != to {
                        allowed_links.insert((*from, *to));
                    }
                }
            }
        }
        self.allowed_links = Some(allowed_links);
    }

    pub fn heal_partitions(&mut self) {
        self.allowed_links = None;
    }

    pub fn replicate_to(
        &mut self,
        targets: impl IntoIterator<Item = NodeId>,
        request: AppendEntriesRequest,
    ) -> EngineResult<BTreeSet<NodeId>> {
        let mut acks = BTreeSet::from([request.leader_id]);
        for target in targets {
            if self.append_entries(target, request.clone())?.success {
                acks.insert(target);
            }
        }
        Ok(acks)
    }

    pub fn replicate_to_best_effort(
        &mut self,
        targets: impl IntoIterator<Item = NodeId>,
        request: AppendEntriesRequest,
    ) -> EngineResult<BTreeSet<NodeId>> {
        let mut acks = BTreeSet::from([request.leader_id]);
        for target in targets {
            if !self.link_allowed(request.leader_id, target) {
                continue;
            }
            if self.append_entries(target, request.clone())?.success {
                acks.insert(target);
            }
        }
        Ok(acks)
    }

    fn link_allowed(&self, from: NodeId, to: NodeId) -> bool {
        match &self.allowed_links {
            Some(links) => links.contains(&(from, to)),
            None => true,
        }
    }
}

impl ReplicationTransport for InMemoryReplicationTransport {
    fn request_vote(&mut self, target: NodeId, request: VoteRequest) -> EngineResult<VoteResponse> {
        if !self.link_allowed(request.candidate_id, target) {
            return Err(EngineError::InvalidOperation);
        }
        let peer = self
            .peers
            .get_mut(&target)
            .ok_or(EngineError::InvalidOperation)?;
        Ok(peer.handle_vote_request(&request))
    }

    fn append_entries(
        &mut self,
        target: NodeId,
        request: AppendEntriesRequest,
    ) -> EngineResult<AppendEntriesResponse> {
        if !self.link_allowed(request.leader_id, target) {
            return Err(EngineError::InvalidOperation);
        }
        let peer = self
            .peers
            .get_mut(&target)
            .ok_or(EngineError::InvalidOperation)?;
        let log = self.logs.entry(target).or_default();
        let success = peer.accept_leader(request.term, request.leader_id)
            && log_matching::append_entries(log, &request).is_some();
        if success {
            let last_index = log.last().map(|entry| entry.index).unwrap_or_default();
            self.commits.insert(
                target,
                log_matching::committed_from_leader(request.leader_commit, last_index),
            );
        }
        let match_index = log.last().map(|entry| entry.index).unwrap_or_default();
        Ok(AppendEntriesResponse {
            term: peer.current_term,
            follower: target,
            success,
            match_index,
        })
    }
}
