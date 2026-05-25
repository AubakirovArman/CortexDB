use std::fs;
use std::path::Path;

use crate::error::{StorageError, StorageResult};

const MAGIC: &[u8; 4] = b"ACS0";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SegmentCell {
    pub cell_id: u64,
    pub payload: Vec<u8>,
}

#[derive(Clone, Debug)]
pub struct SegmentWriter;

#[derive(Clone, Debug)]
pub struct SegmentReader;

impl SegmentWriter {
    pub fn write(path: impl AsRef<Path>, cells: &[SegmentCell]) -> StorageResult<()> {
        let mut out = Vec::new();
        out.extend_from_slice(MAGIC);
        put_u32(&mut out, cells.len() as u32);
        for cell in cells {
            put_u64(&mut out, cell.cell_id);
            put_u32(&mut out, cell.payload.len() as u32);
            out.extend_from_slice(&cell.payload);
        }
        fs::write(path, out)?;
        Ok(())
    }
}

impl SegmentReader {
    pub fn read(path: impl AsRef<Path>) -> StorageResult<Vec<SegmentCell>> {
        let bytes = fs::read(path)?;
        decode_segment(&bytes)
    }
}

fn decode_segment(bytes: &[u8]) -> StorageResult<Vec<SegmentCell>> {
    if bytes.len() < 8 || &bytes[..4] != MAGIC {
        return Err(StorageError::InvalidSegmentFile);
    }
    let mut cursor = 4;
    let count = read_u32(bytes, &mut cursor)? as usize;
    let mut cells = Vec::with_capacity(count);
    for _ in 0..count {
        let cell_id = read_u64(bytes, &mut cursor)?;
        let len = read_u32(bytes, &mut cursor)? as usize;
        let payload = read_bytes(bytes, &mut cursor, len)?.to_vec();
        cells.push(SegmentCell { cell_id, payload });
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
