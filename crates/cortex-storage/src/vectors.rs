use std::collections::BTreeMap;
use std::path::Path;

use crate::atomic::{append_crc32c, verify_crc32c, write_atomic};
use crate::error::{StorageError, StorageResult};
use crate::format::VECTOR_INDEX_MAGIC;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct VectorIndex {
    pub vectors: BTreeMap<u32, Vec<i16>>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct VectorDimensionReport {
    pub vector_count: usize,
    pub expected_dimension: Option<usize>,
    pub mismatched_vectors: usize,
    pub zero_dimension_vectors: usize,
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

    pub fn dimension_report(&self) -> VectorDimensionReport {
        let expected_dimension = self.vectors.values().next().map(Vec::len);
        let mut report = VectorDimensionReport {
            vector_count: self.vectors.len(),
            expected_dimension,
            ..VectorDimensionReport::default()
        };
        for vector in self.vectors.values() {
            if vector.is_empty() {
                report.zero_dimension_vectors += 1;
            }
            if expected_dimension.is_some_and(|dimension| vector.len() != dimension) {
                report.mismatched_vectors += 1;
            }
        }
        report
    }
}

impl VectorDimensionReport {
    pub fn is_valid(self) -> bool {
        self.mismatched_vectors == 0 && self.zero_dimension_vectors == 0
    }

    pub fn summary(self) -> String {
        format!(
            "vectors={} expected_dimension={} mismatched_vectors={} zero_dimension_vectors={}",
            self.vector_count,
            self.expected_dimension
                .map(|value| value.to_string())
                .unwrap_or_else(|| "none".to_owned()),
            self.mismatched_vectors,
            self.zero_dimension_vectors
        )
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
    Ok(i16::from_le_bytes(read_array(bytes, cursor)?))
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
