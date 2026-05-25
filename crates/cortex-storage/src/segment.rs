use std::path::Path;

use crate::atomic::{append_crc32c, verify_crc32c, write_atomic};
use crate::error::{StorageError, StorageResult};
use crate::format::SEGMENT_MAGIC;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SegmentCell {
    pub candidate_id: u32,
    pub cell_id: u64,
    pub created_seq: u64,
    pub deleted_seq: Option<u64>,
    pub payload: Vec<u8>,
}

#[derive(Clone, Debug)]
pub struct SegmentWriter;

#[derive(Clone, Debug)]
pub struct SegmentReader;

impl SegmentWriter {
    pub fn write(path: impl AsRef<Path>, cells: &[SegmentCell]) -> StorageResult<()> {
        let mut out = Vec::new();
        out.extend_from_slice(&SEGMENT_MAGIC);
        put_u32(&mut out, cells.len() as u32);
        let mut ordered = cells.iter().collect::<Vec<_>>();
        ordered.sort_by_key(|cell| cell.candidate_id);
        for cell in ordered {
            put_u64(&mut out, cell.cell_id);
            put_u32(&mut out, cell.candidate_id);
            put_u64(&mut out, cell.created_seq);
            put_u64(&mut out, cell.deleted_seq.unwrap_or(0));
            put_u32(&mut out, cell.payload.len() as u32);
            out.extend_from_slice(&cell.payload);
        }
        append_crc32c(&mut out);
        write_atomic(path.as_ref(), &out)?;
        Ok(())
    }
}

impl SegmentReader {
    pub fn read(path: impl AsRef<Path>) -> StorageResult<Vec<SegmentCell>> {
        let bytes = std::fs::read(path)?;
        decode_segment(&bytes)
    }
}

fn decode_segment(bytes: &[u8]) -> StorageResult<Vec<SegmentCell>> {
    let bytes = verify_crc32c(bytes).ok_or(StorageError::InvalidSegmentFile)?;
    if bytes.len() < 8 || bytes[..4] != SEGMENT_MAGIC {
        return Err(StorageError::InvalidSegmentFile);
    }
    let mut cursor = 4;
    let count = read_u32(bytes, &mut cursor)? as usize;
    let mut cells = Vec::with_capacity(count);
    for _ in 0..count {
        let cell_id = read_u64(bytes, &mut cursor)?;
        let candidate_id = read_u32(bytes, &mut cursor)?;
        let created_seq = read_u64(bytes, &mut cursor)?;
        let deleted_seq = match read_u64(bytes, &mut cursor)? {
            0 => None,
            value => Some(value),
        };
        let len = read_u32(bytes, &mut cursor)? as usize;
        let payload = read_bytes(bytes, &mut cursor, len)?.to_vec();
        cells.push(SegmentCell {
            candidate_id,
            cell_id,
            created_seq,
            deleted_seq,
            payload,
        });
    }
    if cursor != bytes.len() {
        return Err(StorageError::InvalidSegmentFile);
    }
    Ok(cells)
}

fn put_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn put_u64(out: &mut Vec<u8>, value: u64) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn read_u32(bytes: &[u8], cursor: &mut usize) -> StorageResult<u32> {
    let raw = read_bytes(bytes, cursor, 4)?;
    Ok(u32::from_le_bytes(raw.try_into().expect("u32 width")))
}

fn read_u64(bytes: &[u8], cursor: &mut usize) -> StorageResult<u64> {
    let raw = read_bytes(bytes, cursor, 8)?;
    Ok(u64::from_le_bytes(raw.try_into().expect("u64 width")))
}

fn read_bytes<'a>(bytes: &'a [u8], cursor: &mut usize, len: usize) -> StorageResult<&'a [u8]> {
    let end = cursor
        .checked_add(len)
        .ok_or(StorageError::InvalidSegmentFile)?;
    if end > bytes.len() {
        return Err(StorageError::InvalidSegmentFile);
    }
    let value = &bytes[*cursor..end];
    *cursor = end;
    Ok(value)
}
