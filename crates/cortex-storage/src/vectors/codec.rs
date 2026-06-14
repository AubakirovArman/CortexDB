use std::collections::BTreeMap;
use std::path::Path;

use crate::atomic::{append_crc32c, verify_crc32c, write_atomic};
use crate::error::{StorageError, StorageResult};
use crate::format::{LEGACY_VECTOR_INDEX_MAGIC, VECTOR_INDEX_MAGIC};

use super::VectorIndex;

const CURRENT_HEADER_LEN: usize = 12;

pub(super) fn write(index: &VectorIndex, path: &Path) -> StorageResult<()> {
    let out = encode_current_vector_index(index)?;
    write_atomic(path, &out)?;
    Ok(())
}

pub(super) fn read(path: &Path) -> StorageResult<VectorIndex> {
    let bytes = std::fs::read(path)?;
    decode_vector_index(&bytes)
}

fn encode_current_vector_index(index: &VectorIndex) -> StorageResult<Vec<u8>> {
    let report = index.dimension_report();
    if !report.is_valid() {
        return Err(StorageError::InvalidVectorIndexFile);
    }
    let dimension = report.expected_dimension.unwrap_or(0);
    let mut out = Vec::from(&VECTOR_INDEX_MAGIC[..]);
    put_u32(&mut out, checked_u32(index.vectors.len())?);
    put_u32(&mut out, checked_u32(dimension)?);
    for candidate in index.vectors.keys() {
        put_u32(&mut out, *candidate);
    }
    for vector in index.vectors.values() {
        for value in vector {
            put_i16(&mut out, *value);
        }
    }
    append_crc32c(&mut out);
    Ok(out)
}

fn decode_vector_index(bytes: &[u8]) -> StorageResult<VectorIndex> {
    let bytes = verify_crc32c(bytes).ok_or(StorageError::InvalidVectorIndexFile)?;
    if bytes.len() < 8 {
        return Err(StorageError::InvalidVectorIndexFile);
    }
    if bytes[..4] == VECTOR_INDEX_MAGIC {
        decode_current_vector_index(bytes)
    } else if bytes[..4] == LEGACY_VECTOR_INDEX_MAGIC {
        decode_legacy_vector_index(bytes)
    } else {
        Err(StorageError::InvalidVectorIndexFile)
    }
}

fn decode_current_vector_index(bytes: &[u8]) -> StorageResult<VectorIndex> {
    if bytes.len() < CURRENT_HEADER_LEN {
        return Err(StorageError::InvalidVectorIndexFile);
    }
    let mut cursor = 4;
    let count = read_u32(bytes, &mut cursor)? as usize;
    let dimension = read_u32(bytes, &mut cursor)? as usize;
    if count > 0 && dimension == 0 {
        return Err(StorageError::InvalidVectorIndexFile);
    }
    let mut candidates = Vec::with_capacity(count);
    for _ in 0..count {
        candidates.push(read_u32(bytes, &mut cursor)?);
    }
    let row_bytes = checked_row_bytes(dimension)?;
    let expected_len = cursor
        .checked_add(
            count
                .checked_mul(row_bytes)
                .ok_or(StorageError::InvalidVectorIndexFile)?,
        )
        .ok_or(StorageError::InvalidVectorIndexFile)?;
    if expected_len != bytes.len() {
        return Err(StorageError::InvalidVectorIndexFile);
    }
    let mut vectors = BTreeMap::new();
    for candidate in candidates {
        let vector = decode_i16_vec(read_bytes(bytes, &mut cursor, row_bytes)?)?;
        if vectors.insert(candidate, vector).is_some() {
            return Err(StorageError::InvalidVectorIndexFile);
        }
    }
    Ok(VectorIndex { vectors })
}

fn decode_legacy_vector_index(bytes: &[u8]) -> StorageResult<VectorIndex> {
    let mut cursor = 4;
    let count = read_u32(bytes, &mut cursor)? as usize;
    let mut vectors = BTreeMap::new();
    for _ in 0..count {
        let candidate = read_u32(bytes, &mut cursor)?;
        let len = read_u32(bytes, &mut cursor)? as usize;
        let vector = decode_i16_vec(read_bytes(bytes, &mut cursor, checked_row_bytes(len)?)?)?;
        if vector.is_empty() || vectors.insert(candidate, vector).is_some() {
            return Err(StorageError::InvalidVectorIndexFile);
        }
    }
    if cursor != bytes.len() {
        return Err(StorageError::InvalidVectorIndexFile);
    }
    Ok(VectorIndex { vectors })
}

fn decode_i16_vec(bytes: &[u8]) -> StorageResult<Vec<i16>> {
    let mut vector = vec![0_i16; bytes.len() / 2];
    decode_i16_row(bytes, &mut vector)?;
    Ok(vector)
}

pub(super) fn decode_i16_row(bytes: &[u8], out: &mut [i16]) -> StorageResult<()> {
    if bytes.len() != out.len().saturating_mul(2) {
        return Err(StorageError::InvalidVectorIndexFile);
    }
    for (slot, chunk) in out.iter_mut().zip(bytes.chunks_exact(2)) {
        *slot = i16::from_le_bytes([chunk[0], chunk[1]]);
    }
    Ok(())
}

pub(super) fn checked_row_bytes(dimension: usize) -> StorageResult<usize> {
    dimension
        .checked_mul(2)
        .ok_or(StorageError::InvalidVectorIndexFile)
}

fn checked_u32(value: usize) -> StorageResult<u32> {
    u32::try_from(value).map_err(|_| StorageError::InvalidVectorIndexFile)
}

pub(super) fn checked_u64(value: Option<usize>) -> StorageResult<u64> {
    value
        .and_then(|value| u64::try_from(value).ok())
        .ok_or(StorageError::InvalidVectorIndexFile)
}

fn put_i16(out: &mut Vec<u8>, value: i16) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn put_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn read_u32(bytes: &[u8], cursor: &mut usize) -> StorageResult<u32> {
    Ok(u32::from_le_bytes(read_array(bytes, cursor)?))
}

fn read_array<const N: usize>(bytes: &[u8], cursor: &mut usize) -> StorageResult<[u8; N]> {
    read_bytes(bytes, cursor, N)?
        .try_into()
        .map_err(|_| StorageError::InvalidVectorIndexFile)
}

fn read_bytes<'a>(bytes: &'a [u8], cursor: &mut usize, len: usize) -> StorageResult<&'a [u8]> {
    let end = cursor
        .checked_add(len)
        .ok_or(StorageError::InvalidVectorIndexFile)?;
    if end > bytes.len() {
        return Err(StorageError::InvalidVectorIndexFile);
    }
    let value = &bytes[*cursor..end];
    *cursor = end;
    Ok(value)
}
