use crate::error::{EngineError, EngineResult};

use crate::distributed::NodeId;
use cortex_core::CommitSeq;
use cortex_storage::segment::SegmentCell;
use cortex_storage::wal::checksum::crc32c;

use super::{LogIndex, Term};

const SNAPSHOT_SEGMENT_MAGIC: &[u8; 4] = b"CSP1";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SnapshotChunk {
    pub term: Term,
    pub leader_id: NodeId,
    pub leader_commit: LogIndex,
    pub chunk_index: u64,
    pub last: bool,
    pub payload: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SnapshotSegment {
    pub checkpoint_seq: CommitSeq,
    pub cells: Vec<SegmentCell>,
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

pub fn encode_snapshot_segment(snapshot: &SnapshotSegment) -> EngineResult<Vec<u8>> {
    let mut out = Vec::new();
    out.extend_from_slice(SNAPSHOT_SEGMENT_MAGIC);
    put_u64(&mut out, snapshot.checkpoint_seq.0);
    put_u32(&mut out, checked_count(snapshot.cells.len())?);
    let mut cells = snapshot.cells.iter().collect::<Vec<_>>();
    cells.sort_by_key(|cell| cell.candidate_id);
    for cell in cells {
        put_u32(&mut out, cell.candidate_id);
        put_u64(&mut out, cell.cell_id);
        put_u64(&mut out, cell.created_seq);
        put_u64(&mut out, cell.deleted_seq.unwrap_or(0));
        put_u32(&mut out, checked_count(cell.payload.len())?);
        out.extend_from_slice(&cell.payload);
    }
    let checksum = crc32c(&out);
    put_u32(&mut out, checksum);
    Ok(out)
}

pub fn decode_snapshot_segment(bytes: &[u8]) -> EngineResult<SnapshotSegment> {
    let body = verify_snapshot_crc(bytes)?;
    if body.len() < 16 || &body[..4] != SNAPSHOT_SEGMENT_MAGIC {
        return Err(EngineError::InvalidOperation);
    }
    let mut cursor = 4;
    let checkpoint_seq = CommitSeq(read_u64(body, &mut cursor)?);
    let count = read_u32(body, &mut cursor)? as usize;
    let mut cells = Vec::with_capacity(count);
    for _ in 0..count {
        let candidate_id = read_u32(body, &mut cursor)?;
        let cell_id = read_u64(body, &mut cursor)?;
        let created_seq = read_u64(body, &mut cursor)?;
        let deleted_seq = match read_u64(body, &mut cursor)? {
            0 => None,
            value => Some(value),
        };
        let payload_len = read_u32(body, &mut cursor)? as usize;
        let payload = read_bytes(body, &mut cursor, payload_len)?.to_vec();
        cells.push(SegmentCell {
            candidate_id,
            cell_id,
            created_seq,
            deleted_seq,
            payload,
        });
    }
    if cursor != body.len() {
        return Err(EngineError::InvalidOperation);
    }
    Ok(SnapshotSegment {
        checkpoint_seq,
        cells,
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

fn verify_snapshot_crc(bytes: &[u8]) -> EngineResult<&[u8]> {
    if bytes.len() < 4 {
        return Err(EngineError::InvalidOperation);
    }
    let body_len = bytes.len() - 4;
    let expected = u32::from_le_bytes(
        bytes[body_len..]
            .try_into()
            .map_err(|_| EngineError::InvalidOperation)?,
    );
    if crc32c(&bytes[..body_len]) != expected {
        return Err(EngineError::InvalidOperation);
    }
    Ok(&bytes[..body_len])
}

fn checked_count(len: usize) -> EngineResult<u32> {
    u32::try_from(len).map_err(|_| EngineError::InvalidOperation)
}

fn put_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn put_u64(out: &mut Vec<u8>, value: u64) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn read_u32(bytes: &[u8], cursor: &mut usize) -> EngineResult<u32> {
    let raw = read_bytes(bytes, cursor, 4)?;
    Ok(u32::from_le_bytes(
        raw.try_into().map_err(|_| EngineError::InvalidOperation)?,
    ))
}

fn read_u64(bytes: &[u8], cursor: &mut usize) -> EngineResult<u64> {
    let raw = read_bytes(bytes, cursor, 8)?;
    Ok(u64::from_le_bytes(
        raw.try_into().map_err(|_| EngineError::InvalidOperation)?,
    ))
}

fn read_bytes<'a>(bytes: &'a [u8], cursor: &mut usize, len: usize) -> EngineResult<&'a [u8]> {
    let end = cursor
        .checked_add(len)
        .ok_or(EngineError::InvalidOperation)?;
    if end > bytes.len() {
        return Err(EngineError::InvalidOperation);
    }
    let value = &bytes[*cursor..end];
    *cursor = end;
    Ok(value)
}
