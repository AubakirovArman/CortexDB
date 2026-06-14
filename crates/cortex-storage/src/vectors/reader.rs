use std::cmp::Reverse;
use std::collections::BTreeSet;
use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::path::Path;

use crate::error::{StorageError, StorageResult};
use crate::format::VECTOR_INDEX_MAGIC;

use super::codec::{checked_row_bytes, checked_u64, decode_i16_row, read};
use super::VectorIndex;

const CURRENT_HEADER_LEN: usize = 12;
const CRC_LEN: u64 = 4;

#[derive(Debug)]
pub struct VectorIndexReader {
    inner: VectorIndexReaderInner,
}

#[derive(Debug)]
enum VectorIndexReaderInner {
    Contiguous(ContiguousVectorReader),
    Legacy(VectorIndex),
}

#[derive(Debug)]
struct ContiguousVectorReader {
    file: File,
    candidates: Vec<u32>,
    dimension: usize,
    vectors_offset: u64,
}

impl VectorIndexReader {
    pub fn open(path: impl AsRef<Path>) -> StorageResult<Self> {
        let path = path.as_ref();
        let mut file = File::open(path)?;
        let mut magic = [0_u8; 4];
        file.read_exact(&mut magic)?;
        if magic == VECTOR_INDEX_MAGIC {
            return Ok(Self {
                inner: VectorIndexReaderInner::Contiguous(open_current_reader(file)?),
            });
        }
        Ok(Self {
            inner: VectorIndexReaderInner::Legacy(read(path)?),
        })
    }

    pub fn dimension(&self) -> Option<usize> {
        match &self.inner {
            VectorIndexReaderInner::Contiguous(reader) => {
                (reader.dimension > 0).then_some(reader.dimension)
            }
            VectorIndexReaderInner::Legacy(index) => index.dimension_report().expected_dimension,
        }
    }

    pub fn vector_count(&self) -> usize {
        match &self.inner {
            VectorIndexReaderInner::Contiguous(reader) => reader.candidates.len(),
            VectorIndexReaderInner::Legacy(index) => index.vectors.len(),
        }
    }

    pub fn scan_scores<F>(
        &mut self,
        allowed: &BTreeSet<u32>,
        limit: usize,
        score: F,
    ) -> StorageResult<Vec<(u32, u64)>>
    where
        F: FnMut(&[i16]) -> Option<u64>,
    {
        if limit == 0 || allowed.is_empty() {
            return Ok(Vec::new());
        }
        match &mut self.inner {
            VectorIndexReaderInner::Contiguous(reader) => reader.scan_scores(allowed, limit, score),
            VectorIndexReaderInner::Legacy(index) => scan_legacy(index, allowed, limit, score),
        }
    }
}

impl ContiguousVectorReader {
    fn scan_scores<F>(
        &mut self,
        allowed: &BTreeSet<u32>,
        limit: usize,
        score: F,
    ) -> StorageResult<Vec<(u32, u64)>>
    where
        F: FnMut(&[i16]) -> Option<u64>,
    {
        if allowed.len().saturating_mul(2) >= self.candidates.len() {
            return self.scan_scores_sequential(allowed, limit, score);
        }
        self.scan_scores_sparse(allowed, limit, score)
    }

    fn scan_scores_sparse<F>(
        &mut self,
        allowed: &BTreeSet<u32>,
        limit: usize,
        mut score: F,
    ) -> StorageResult<Vec<(u32, u64)>>
    where
        F: FnMut(&[i16]) -> Option<u64>,
    {
        let row_bytes = checked_row_bytes(self.dimension)?;
        let mut raw = vec![0_u8; row_bytes];
        let mut vector = vec![0_i16; self.dimension];
        let mut scores = Vec::new();
        for (row, candidate) in self.candidates.iter().enumerate() {
            if !allowed.contains(candidate) {
                continue;
            }
            let offset = self
                .vectors_offset
                .checked_add(checked_u64(row.checked_mul(row_bytes))?)
                .ok_or(StorageError::InvalidVectorIndexFile)?;
            self.file.seek(SeekFrom::Start(offset))?;
            self.file.read_exact(&mut raw)?;
            decode_i16_row(&raw, &mut vector)?;
            if let Some(value) = score(&vector) {
                push_score(&mut scores, *candidate, value, limit);
            }
        }
        Ok(finish_scores(scores, limit))
    }

    fn scan_scores_sequential<F>(
        &self,
        allowed: &BTreeSet<u32>,
        limit: usize,
        mut score: F,
    ) -> StorageResult<Vec<(u32, u64)>>
    where
        F: FnMut(&[i16]) -> Option<u64>,
    {
        let row_bytes = checked_row_bytes(self.dimension)?;
        let mut file = self.file.try_clone()?;
        file.seek(SeekFrom::Start(self.vectors_offset))?;
        let mut reader = BufReader::new(file);
        let mut raw = vec![0_u8; row_bytes];
        let mut vector = vec![0_i16; self.dimension];
        let mut scores = Vec::new();
        for candidate in &self.candidates {
            reader.read_exact(&mut raw)?;
            if !allowed.contains(candidate) {
                continue;
            }
            decode_i16_row(&raw, &mut vector)?;
            if let Some(value) = score(&vector) {
                push_score(&mut scores, *candidate, value, limit);
            }
        }
        Ok(finish_scores(scores, limit))
    }
}

fn scan_legacy<F>(
    index: &VectorIndex,
    allowed: &BTreeSet<u32>,
    limit: usize,
    mut score: F,
) -> StorageResult<Vec<(u32, u64)>>
where
    F: FnMut(&[i16]) -> Option<u64>,
{
    let mut scores = Vec::new();
    for (candidate, vector) in &index.vectors {
        if allowed.contains(candidate) {
            if let Some(value) = score(vector) {
                push_score(&mut scores, *candidate, value, limit);
            }
        }
    }
    Ok(finish_scores(scores, limit))
}

fn open_current_reader(mut file: File) -> StorageResult<ContiguousVectorReader> {
    let len = file.metadata()?.len();
    let mut header = [0_u8; CURRENT_HEADER_LEN];
    file.seek(SeekFrom::Start(0))?;
    file.read_exact(&mut header)?;
    if header[..4] != VECTOR_INDEX_MAGIC {
        return Err(StorageError::InvalidVectorIndexFile);
    }
    let count = u32::from_le_bytes([header[4], header[5], header[6], header[7]]) as usize;
    let dimension = u32::from_le_bytes([header[8], header[9], header[10], header[11]]) as usize;
    if count > 0 && dimension == 0 {
        return Err(StorageError::InvalidVectorIndexFile);
    }
    let candidate_bytes = count
        .checked_mul(4)
        .ok_or(StorageError::InvalidVectorIndexFile)?;
    let row_bytes = checked_row_bytes(dimension)?;
    let vectors_offset = checked_u64(CURRENT_HEADER_LEN.checked_add(candidate_bytes))?;
    let vector_bytes = checked_u64(count.checked_mul(row_bytes))?;
    let expected_len = vectors_offset
        .checked_add(vector_bytes)
        .and_then(|value| value.checked_add(CRC_LEN))
        .ok_or(StorageError::InvalidVectorIndexFile)?;
    if len != expected_len {
        return Err(StorageError::InvalidVectorIndexFile);
    }
    let mut candidates_raw = vec![0_u8; candidate_bytes];
    file.read_exact(&mut candidates_raw)?;
    let mut candidates = Vec::with_capacity(count);
    let mut seen = BTreeSet::new();
    for chunk in candidates_raw.chunks_exact(4) {
        let candidate = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
        if !seen.insert(candidate) {
            return Err(StorageError::InvalidVectorIndexFile);
        }
        candidates.push(candidate);
    }
    Ok(ContiguousVectorReader {
        file,
        candidates,
        dimension,
        vectors_offset,
    })
}

fn push_score(scores: &mut Vec<(u32, u64)>, candidate: u32, score: u64, limit: usize) {
    scores.push((candidate, score));
    if scores.len() > limit.saturating_mul(4).max(64) {
        truncate_scores(scores, limit);
    }
}

fn finish_scores(mut scores: Vec<(u32, u64)>, limit: usize) -> Vec<(u32, u64)> {
    truncate_scores(&mut scores, limit);
    scores
}

fn truncate_scores(scores: &mut Vec<(u32, u64)>, limit: usize) {
    scores.sort_by_key(|(candidate, score)| (Reverse(*score), *candidate));
    scores.truncate(limit);
}
