use std::collections::BTreeMap;
use std::path::Path;

use crate::atomic::{append_crc32c, verify_crc32c, write_atomic};
use crate::error::{StorageError, StorageResult};
use crate::format::VECTOR_INDEX_MAGIC;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct VectorIndex {
    pub vectors: BTreeMap<u32, Vec<i16>>,
}

impl VectorIndex {
    pub fn write(&self, path: impl AsRef<Path>) -> StorageResult<()> {
        let mut out = Vec::from(&VECTOR_INDEX_MAGIC[..]);
        put_u32(&mut out, self.vectors.len() as u32);
        for (candidate, vector) in &self.vectors {
            put_u32(&mut out, *candidate);
            put_u32(&mut out, vector.len() as u32);
            for value in vector {
                put_i16(&mut out, *value);
            }
        }
        append_crc32c(&mut out);
        write_atomic(path.as_ref(), &out)?;
        Ok(())
    }

    pub fn read(path: impl AsRef<Path>) -> StorageResult<Self> {
        let bytes = std::fs::read(path)?;
        decode_vector_index(&bytes)
    }
}

fn decode_vector_index(bytes: &[u8]) -> StorageResult<VectorIndex> {
    let bytes = verify_crc32c(bytes).ok_or(StorageError::InvalidVectorIndexFile)?;
    if bytes.len() < 8 || bytes[..4] != VECTOR_INDEX_MAGIC {
        return Err(StorageError::InvalidVectorIndexFile);
    }
    let mut cursor = 4;
    let count = read_u32(bytes, &mut cursor)? as usize;
    let mut vectors = BTreeMap::new();
    for _ in 0..count {
        let candidate = read_u32(bytes, &mut cursor)?;
        let len = read_u32(bytes, &mut cursor)? as usize;
        let mut vector = Vec::with_capacity(len);
        for _ in 0..len {
            vector.push(read_i16(bytes, &mut cursor)?);
        }
        if vector.is_empty() || vectors.insert(candidate, vector).is_some() {
            return Err(StorageError::InvalidVectorIndexFile);
        }
    }
    if cursor != bytes.len() {
        return Err(StorageError::InvalidVectorIndexFile);
    }
    Ok(VectorIndex { vectors })
}

fn put_i16(out: &mut Vec<u8>, value: i16) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn put_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn read_i16(bytes: &[u8], cursor: &mut usize) -> StorageResult<i16> {
    let raw = read_bytes(bytes, cursor, 2)?;
    Ok(i16::from_le_bytes(raw.try_into().expect("i16 width")))
}

fn read_u32(bytes: &[u8], cursor: &mut usize) -> StorageResult<u32> {
    let raw = read_bytes(bytes, cursor, 4)?;
    Ok(u32::from_le_bytes(raw.try_into().expect("u32 width")))
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
