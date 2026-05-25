use std::collections::{BTreeMap, BTreeSet};

use crate::distributed::NodeId;
use crate::error::{EngineError, EngineResult};

use super::election::{ElectionState, VoteRequest, VoteResponse};
use super::{LogIndex, ReplicatedEntry, Term};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AppendEntriesRequest {
    pub term: Term,
    pub leader_id: NodeId,
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
}

impl InMemoryReplicationTransport {
    pub fn register_peer(&mut self, state: ElectionState) {
        self.logs.entry(state.local_node).or_default();
        self.peers.insert(state.local_node, state);
    }

    pub fn peer_log(&self, node: NodeId) -> Option<&[ReplicatedEntry]> {
        self.logs.get(&node).map(Vec::as_slice)
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
}

impl ReplicationTransport for InMemoryReplicationTransport {
    fn request_vote(&mut self, target: NodeId, request: VoteRequest) -> EngineResult<VoteResponse> {
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
        let peer = self
            .peers
            .get_mut(&target)
            .ok_or(EngineError::InvalidOperation)?;
        let success = peer.accept_leader(request.term, request.leader_id);
        let log = self.logs.entry(target).or_default();
        if success {
            for entry in request.entries {
                if !log.iter().any(|existing| existing.index == entry.index) {
                    log.push(entry);
                }
            }
            log.sort_by_key(|entry| entry.index);
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
