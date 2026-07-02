use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::net::TcpStream;

use crate::distributed::NodeId;
use crate::error::{EngineError, EngineResult};

use super::election::{ElectionRole, ElectionState, VoteRequest, VoteResponse};
use super::log_matching;
use super::snapshot::{
    decode_snapshot_chunk, hex_decode, hex_encode, parse_bool, parse_u64, SnapshotChunk,
};
use super::transport::{AppendEntriesRequest, AppendEntriesResponse, ReplicationTransport};
use super::{LogIndex, ReplicatedEntry, Term};

#[derive(Clone, Debug, Default)]
pub struct TcpReplicationTransport {
    peers: BTreeMap<NodeId, String>,
    token: Option<String>,
}

impl TcpReplicationTransport {
    pub fn new(peers: BTreeMap<NodeId, String>) -> Self {
        Self { peers, token: None }
    }

    pub fn with_token(peers: BTreeMap<NodeId, String>, token: String) -> Self {
        Self {
            peers,
            token: Some(token),
        }
    }

    fn roundtrip(&self, target: NodeId, frame: String) -> EngineResult<String> {
        let addr = self
            .peers
            .get(&target)
            .ok_or(EngineError::InvalidOperation)?;
        let mut stream = TcpStream::connect(addr)?;
        let frame = self
            .token
            .as_ref()
            .map(|token| format!("AUTH {token} {frame}"))
            .unwrap_or(frame);
        stream.write_all(frame.as_bytes())?;
        stream.shutdown(std::net::Shutdown::Write)?;
        let mut response = String::new();
        stream.read_to_string(&mut response)?;
        Ok(response)
    }

    pub fn send_snapshot_chunk(&self, target: NodeId, chunk: &SnapshotChunk) -> EngineResult<u64> {
        let response = self.roundtrip(target, encode_snapshot_frame(chunk))?;
        decode_snapshot_response(&response)
    }
}

impl ReplicationTransport for TcpReplicationTransport {
    fn request_vote(&mut self, target: NodeId, request: VoteRequest) -> EngineResult<VoteResponse> {
        decode_vote_response(&self.roundtrip(target, encode_vote_request(&request))?)
    }

    fn append_entries(
        &mut self,
        target: NodeId,
        request: AppendEntriesRequest,
    ) -> EngineResult<AppendEntriesResponse> {
        decode_append_response(&self.roundtrip(target, encode_append_request(&request))?)
    }
}

pub fn handle_replication_frame(
    state: &mut ElectionState,
    log: &mut Vec<ReplicatedEntry>,
    frame: &str,
) -> EngineResult<String> {
    let mut snapshot = Vec::new();
    handle_authenticated_replication_frame(state, log, &mut snapshot, None, frame)
}

pub fn handle_authenticated_replication_frame(
    state: &mut ElectionState,
    log: &mut Vec<ReplicatedEntry>,
    snapshot: &mut Vec<u8>,
    expected_token: Option<&str>,
    frame: &str,
) -> EngineResult<String> {
    let frame = authenticated_payload(expected_token, frame)?;
    let parts = frame.split_whitespace().collect::<Vec<_>>();
    match parts.as_slice() {
        ["VOTE", term, candidate, last_index, last_term] => {
            let request = VoteRequest {
                term: Term(parse_u64(term)?),
                candidate_id: NodeId(parse_u64(candidate)?),
                last_log_index: LogIndex(parse_u64(last_index)?),
                last_log_term: Term(parse_u64(last_term)?),
            };
            Ok(encode_vote_response(&state.handle_vote_request(&request)))
        }
        ["STATUS"] => Ok(status_response(state)),
        ["APPEND", term, leader, prev_index, prev_term, commit, rest @ ..] => {
            let request = AppendEntriesRequest {
                term: Term(parse_u64(term)?),
                leader_id: NodeId(parse_u64(leader)?),
                prev_log_index: LogIndex(parse_u64(prev_index)?),
                prev_log_term: Term(parse_u64(prev_term)?),
                leader_commit: LogIndex(parse_u64(commit)?),
                entries: decode_entries(rest)?,
            };
            let success = state.accept_leader(request.term, request.leader_id)
                && log_matching::append_entries(log, &request).is_some();
            let match_index = log.last().map(|entry| entry.index).unwrap_or_default();
            Ok(append_response(&AppendEntriesResponse {
                term: state.current_term,
                follower: state.local_node,
                success,
                match_index,
            }))
        }
        ["SNAPSHOT", rest @ ..] => {
            let chunk = decode_snapshot_chunk(rest)?;
            if !state.accept_leader(chunk.term, chunk.leader_id) {
                return Ok(snapshot_response(state.current_term, false, 0));
            }
            if chunk.chunk_index == 0 {
                snapshot.clear();
            } else if snapshot.is_empty() {
                return Ok(snapshot_response(state.current_term, false, 0));
            }
            snapshot.extend_from_slice(&chunk.payload);
            Ok(snapshot_response(state.current_term, true, snapshot.len()))
        }
        _ => Err(EngineError::InvalidOperation),
    }
}

fn status_response(state: &ElectionState) -> String {
    let role = match state.role {
        ElectionRole::Follower => "follower",
        ElectionRole::Candidate => "candidate",
        ElectionRole::Leader => "leader",
    };
    let leader = state.leader.map(|node| node.0).unwrap_or(0);
    format!(
        "STATUS_RESP {} {} {} {}\n",
        state.current_term.0, state.local_node.0, role, leader
    )
}

pub(super) fn authenticated_payload<'a>(
    expected_token: Option<&str>,
    frame: &'a str,
) -> EngineResult<&'a str> {
    match expected_token {
        Some(token) => {
            let frame = frame.trim_start();
            let Some(rest) = frame.strip_prefix("AUTH ") else {
                return Err(EngineError::InvalidOperation);
            };
            let Some((actual, payload)) = rest.split_once(' ') else {
                return Err(EngineError::InvalidOperation);
            };
            if actual == token {
                Ok(payload)
            } else {
                Err(EngineError::InvalidOperation)
            }
        }
        None => Ok(frame),
    }
}

fn encode_vote_request(request: &VoteRequest) -> String {
    format!(
        "VOTE {} {} {} {}\n",
        request.term.0, request.candidate_id.0, request.last_log_index.0, request.last_log_term.0
    )
}

fn encode_vote_response(response: &VoteResponse) -> String {
    format!(
        "VOTE_RESP {} {} {}\n",
        response.term.0, response.voter.0, response.vote_granted as u8
    )
}

fn decode_vote_response(frame: &str) -> EngineResult<VoteResponse> {
    let parts = frame.split_whitespace().collect::<Vec<_>>();
    let ["VOTE_RESP", term, voter, granted] = parts.as_slice() else {
        return Err(EngineError::InvalidOperation);
    };
    Ok(VoteResponse {
        term: Term(parse_u64(term)?),
        voter: NodeId(parse_u64(voter)?),
        vote_granted: parse_bool(granted)?,
    })
}

fn encode_append_request(request: &AppendEntriesRequest) -> String {
    let mut out = format!(
        "APPEND {} {} {} {} {}",
        request.term.0,
        request.leader_id.0,
        request.prev_log_index.0,
        request.prev_log_term.0,
        request.leader_commit.0
    );
    for entry in &request.entries {
        out.push_str(&format!(
            " {}:{}:{}",
            entry.term.0,
            entry.index.0,
            hex_encode(&entry.payload)
        ));
    }
    out.push('\n');
    out
}

fn append_response(response: &AppendEntriesResponse) -> String {
    format!(
        "APPEND_RESP {} {} {} {}\n",
        response.term.0, response.follower.0, response.success as u8, response.match_index.0
    )
}

fn snapshot_response(term: Term, success: bool, bytes: usize) -> String {
    format!("SNAPSHOT_RESP {} {} {}\n", term.0, success as u8, bytes)
}

fn decode_append_response(frame: &str) -> EngineResult<AppendEntriesResponse> {
    let parts = frame.split_whitespace().collect::<Vec<_>>();
    let ["APPEND_RESP", term, follower, success, match_index] = parts.as_slice() else {
        return Err(EngineError::InvalidOperation);
    };
    Ok(AppendEntriesResponse {
        term: Term(parse_u64(term)?),
        follower: NodeId(parse_u64(follower)?),
        success: parse_bool(success)?,
        match_index: LogIndex(parse_u64(match_index)?),
    })
}

fn decode_entries(parts: &[&str]) -> EngineResult<Vec<ReplicatedEntry>> {
    parts
        .iter()
        .map(|part| {
            let fields = part.split(':').collect::<Vec<_>>();
            let [term, index, payload] = fields.as_slice() else {
                return Err(EngineError::InvalidOperation);
            };
            Ok(ReplicatedEntry {
                term: Term(parse_u64(term)?),
                index: LogIndex(parse_u64(index)?),
                payload: hex_decode(payload)?,
            })
        })
        .collect()
}

pub fn encode_snapshot_frame(chunk: &SnapshotChunk) -> String {
    super::snapshot::encode_snapshot_chunk(chunk)
}

fn decode_snapshot_response(frame: &str) -> EngineResult<u64> {
    let parts = frame.split_whitespace().collect::<Vec<_>>();
    let ["SNAPSHOT_RESP", _, success, bytes] = parts.as_slice() else {
        return Err(EngineError::InvalidOperation);
    };
    if !parse_bool(success)? {
        return Err(EngineError::InvalidOperation);
    }
    parse_u64(bytes)
}
