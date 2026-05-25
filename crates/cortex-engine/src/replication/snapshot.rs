use crate::error::{EngineError, EngineResult};

use crate::distributed::NodeId;

use super::{LogIndex, Term};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SnapshotChunk {
    pub term: Term,
    pub leader_id: NodeId,
    pub leader_commit: LogIndex,
    pub chunk_index: u64,
    pub last: bool,
    pub payload: Vec<u8>,
}

pub fn encode_snapshot_chunk(chunk: &SnapshotChunk) -> String {
    format!(
        "SNAPSHOT {} {} {} {} {} {}\n",
        chunk.term.0,
        chunk.leader_id.0,
        chunk.leader_commit.0,
        chunk.chunk_index,
        chunk.last as u8,
        hex_encode(&chunk.payload)
    )
}

pub fn decode_snapshot_chunk(parts: &[&str]) -> EngineResult<SnapshotChunk> {
    let [term, leader, commit, chunk_index, last, payload] = parts else {
        return Err(EngineError::InvalidOperation);
    };
    Ok(SnapshotChunk {
        term: Term(parse_u64(term)?),
        leader_id: NodeId(parse_u64(leader)?),
        leader_commit: LogIndex(parse_u64(commit)?),
        chunk_index: parse_u64(chunk_index)?,
        last: parse_bool(last)?,
        payload: hex_decode(payload)?,
    })
}

pub(crate) fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

pub(crate) fn hex_decode(value: &str) -> EngineResult<Vec<u8>> {
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

pub(crate) fn parse_u64(value: &str) -> EngineResult<u64> {
    value.parse().map_err(|_| EngineError::InvalidOperation)
}

pub(crate) fn parse_bool(value: &str) -> EngineResult<bool> {
    match value {
        "0" => Ok(false),
        "1" => Ok(true),
        _ => Err(EngineError::InvalidOperation),
    }
}
