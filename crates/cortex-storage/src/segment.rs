use std::collections::BTreeMap;
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SegmentCellRef<'a> {
    pub candidate_id: u32,
    pub cell_id: u64,
    pub created_seq: u64,
    pub deleted_seq: Option<u64>,
    pub payload: &'a [u8],
}

impl<'a> From<&'a SegmentCell> for SegmentCellRef<'a> {
    fn from(cell: &'a SegmentCell) -> Self {
        Self {
            candidate_id: cell.candidate_id,
            cell_id: cell.cell_id,
            created_seq: cell.created_seq,
            deleted_seq: cell.deleted_seq,
            payload: &cell.payload,
        }
    }
}

/// Lightweight per-cell entry that omits the payload. Index-rebuild paths only
/// need the candidate/cell identity and liveness, so decoding this avoids
/// copying multi-gigabyte payload bytes for large checkpointed corpora.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SegmentCandidateEntry {
    pub candidate_id: u32,
    pub cell_id: u64,
    pub deleted: bool,
}

#[derive(Clone, Debug)]
pub struct SegmentWriter;

#[derive(Clone, Debug)]
pub struct SegmentReader;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SegmentLookup {
    cells: Vec<SegmentCell>,
    by_candidate: BTreeMap<u32, usize>,
    by_cell_id: BTreeMap<u64, usize>,
}

impl SegmentWriter {
    pub fn write(path: impl AsRef<Path>, cells: &[SegmentCell]) -> StorageResult<()> {
        let refs = cells.iter().map(SegmentCellRef::from).collect::<Vec<_>>();
        Self::write_refs(path, &refs)
    }

    pub fn write_refs(path: impl AsRef<Path>, cells: &[SegmentCellRef<'_>]) -> StorageResult<()> {
        let mut out = Vec::new();
        out.extend_from_slice(&SEGMENT_MAGIC);
        put_u32(&mut out, cells.len() as u32);
        let mut ordered = cells.to_vec();
        ordered.sort_by_key(|cell| cell.candidate_id);
        for cell in ordered {
            put_u64(&mut out, cell.cell_id);
            put_u32(&mut out, cell.candidate_id);
            put_u64(&mut out, cell.created_seq);
            put_u64(&mut out, cell.deleted_seq.unwrap_or(0));
            put_u32(&mut out, cell.payload.len() as u32);
            out.extend_from_slice(cell.payload);
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

    pub fn read_lookup(path: impl AsRef<Path>) -> StorageResult<SegmentLookup> {
        SegmentLookup::new(Self::read(path)?)
    }

    /// Read only candidate/cell identity and liveness, skipping payload bytes.
    pub fn read_candidate_entries(
        path: impl AsRef<Path>,
    ) -> StorageResult<Vec<SegmentCandidateEntry>> {
        let bytes = std::fs::read(path)?;
        decode_segment_candidate_entries(&bytes)
    }
}

impl SegmentLookup {
    pub fn new(cells: Vec<SegmentCell>) -> StorageResult<Self> {
        let mut by_candidate = BTreeMap::new();
        let mut by_cell_id = BTreeMap::new();
        for (index, cell) in cells.iter().enumerate() {
            if by_candidate.insert(cell.candidate_id, index).is_some()
                || by_cell_id.insert(cell.cell_id, index).is_some()
            {
                return Err(StorageError::InvalidSegmentFile);
            }
        }
        Ok(Self {
            cells,
            by_candidate,
            by_cell_id,
        })
    }

    pub fn cell_by_candidate(&self, candidate_id: u32) -> Option<&SegmentCell> {
        self.by_candidate
            .get(&candidate_id)
            .and_then(|index| self.cells.get(*index))
    }

    pub fn cell_by_cell_id(&self, cell_id: u64) -> Option<&SegmentCell> {
        self.by_cell_id
            .get(&cell_id)
            .and_then(|index| self.cells.get(*index))
    }

    pub fn cells(&self) -> &[SegmentCell] {
        &self.cells
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

fn decode_segment_candidate_entries(bytes: &[u8]) -> StorageResult<Vec<SegmentCandidateEntry>> {
    let bytes = verify_crc32c(bytes).ok_or(StorageError::InvalidSegmentFile)?;
    if bytes.len() < 8 || bytes[..4] != SEGMENT_MAGIC {
        return Err(StorageError::InvalidSegmentFile);
    }
    let mut cursor = 4;
    let count = read_u32(bytes, &mut cursor)? as usize;
    let mut entries = Vec::with_capacity(count);
    for _ in 0..count {
        let cell_id = read_u64(bytes, &mut cursor)?;
        let candidate_id = read_u32(bytes, &mut cursor)?;
        let _created_seq = read_u64(bytes, &mut cursor)?;
        let deleted = read_u64(bytes, &mut cursor)? != 0;
        let len = read_u32(bytes, &mut cursor)? as usize;
        // Skip the payload without copying it.
        read_bytes(bytes, &mut cursor, len)?;
        entries.push(SegmentCandidateEntry {
            candidate_id,
            cell_id,
            deleted,
        });
    }
    if cursor != bytes.len() {
        return Err(StorageError::InvalidSegmentFile);
    }
    Ok(entries)
}

fn put_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn put_u64(out: &mut Vec<u8>, value: u64) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn read_u32(bytes: &[u8], cursor: &mut usize) -> StorageResult<u32> {
    Ok(u32::from_le_bytes(read_array(bytes, cursor)?))
}

fn read_u64(bytes: &[u8], cursor: &mut usize) -> StorageResult<u64> {
    Ok(u64::from_le_bytes(read_array(bytes, cursor)?))
}

fn read_array<const N: usize>(bytes: &[u8], cursor: &mut usize) -> StorageResult<[u8; N]> {
    read_bytes(bytes, cursor, N)?
        .try_into()
        .map_err(|_| StorageError::InvalidSegmentFile)
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

#[cfg(test)]
mod tests {
    use super::{SegmentCandidateEntry, SegmentCell, SegmentReader, SegmentWriter};

    fn sample_cells() -> Vec<SegmentCell> {
        vec![
            SegmentCell {
                candidate_id: 1,
                cell_id: 10,
                created_seq: 1,
                deleted_seq: None,
                payload: b"first payload body".to_vec(),
            },
            SegmentCell {
                candidate_id: 2,
                cell_id: 20,
                created_seq: 2,
                deleted_seq: Some(5),
                payload: b"second, tombstoned".to_vec(),
            },
        ]
    }

    #[test]
    fn read_candidate_entries_matches_full_read_without_payload() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("segment.acs");
        SegmentWriter::write(&path, &sample_cells()).unwrap();

        let full = SegmentReader::read(&path).unwrap();
        let entries = SegmentReader::read_candidate_entries(&path).unwrap();

        let expected: Vec<SegmentCandidateEntry> = full
            .iter()
            .map(|cell| SegmentCandidateEntry {
                candidate_id: cell.candidate_id,
                cell_id: cell.cell_id,
                deleted: cell.deleted_seq.is_some(),
            })
            .collect();
        assert_eq!(entries, expected);
    }
}
