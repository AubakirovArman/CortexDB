use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::net::TcpStream;

use crate::distributed::NodeId;
use crate::error::{EngineError, EngineResult};

use super::election::{ElectionState, VoteRequest, VoteResponse};
use super::transport::{AppendEntriesRequest, AppendEntriesResponse, ReplicationTransport};
use super::{LogIndex, ReplicatedEntry, Term};

#[derive(Clone, Debug, Default)]
pub struct TcpReplicationTransport {
    peers: BTreeMap<NodeId, String>,
}

impl TcpReplicationTransport {
    pub fn new(peers: BTreeMap<NodeId, String>) -> Self {
        Self { peers }
    }

    fn roundtrip(&self, target: NodeId, frame: String) -> EngineResult<String> {
        let addr = self
            .peers
            .get(&target)
            .ok_or(EngineError::InvalidOperation)?;
        let mut stream = TcpStream::connect(addr)?;
        stream.write_all(frame.as_bytes())?;
        stream.shutdown(std::net::Shutdown::Write)?;
        let mut response = String::new();
        stream.read_to_string(&mut response)?;
        Ok(response)
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
        ["APPEND", term, leader, commit, rest @ ..] => {
            let request = AppendEntriesRequest {
                term: Term(parse_u64(term)?),
                leader_id: NodeId(parse_u64(leader)?),
                leader_commit: LogIndex(parse_u64(commit)?),
                entries: decode_entries(rest)?,
            };
            let success = state.accept_leader(request.term, request.leader_id);
            if success {
                for entry in request.entries {
                    if !log.iter().any(|existing| existing.index == entry.index) {
                        log.push(entry);
                    }
                }
                log.sort_by_key(|entry| entry.index);
            }
            let match_index = log.last().map(|entry| entry.index).unwrap_or_default();
            Ok(encode_append_response(&AppendEntriesResponse {
                term: state.current_term,
                follower: state.local_node,
                success,
                match_index,
            }))
        }
        _ => Err(EngineError::InvalidOperation),
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
        "APPEND {} {} {}",
        request.term.0, request.leader_id.0, request.leader_commit.0
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

fn encode_append_response(response: &AppendEntriesResponse) -> String {
    format!(
        "APPEND_RESP {} {} {} {}\n",
        response.term.0, response.follower.0, response.success as u8, response.match_index.0
    )
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

fn parse_u64(value: &str) -> EngineResult<u64> {
    value.parse().map_err(|_| EngineError::InvalidOperation)
}

fn parse_bool(value: &str) -> EngineResult<bool> {
    match value {
        "0" => Ok(false),
        "1" => Ok(true),
        _ => Err(EngineError::InvalidOperation),
    }
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn hex_decode(value: &str) -> EngineResult<Vec<u8>> {
    if value.len() & 1 == 1 {
        return Err(EngineError::InvalidOperation);
    }
    value
        .as_bytes()
        .chunks(2)
        .map(|chunk| {
            let text = std::str::from_utf8(chunk).map_err(|_| EngineError::InvalidOperation)?;
            u8::from_str_radix(text, 16).map_err(|_| EngineError::InvalidOperation)
        })
        .collect()
}
