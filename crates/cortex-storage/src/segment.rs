use std::collections::BTreeMap;
use std::path::Path;

use crate::atomic::write_atomic;
use crate::error::{StorageError, StorageResult};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SegmentCell {
    pub candidate_id: u32,
    pub cell_id: u64,
    pub created_seq: u64,
    pub deleted_seq: Option<u64>,
    pub payload: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SegmentCellRef<'a> {
    pub candidate_id: u32,
    pub cell_id: u64,
    pub created_seq: u64,
    pub deleted_seq: Option<u64>,
    pub descriptor: Option<Vec<u8>>,
    pub payload: &'a [u8],
}

impl<'a> From<&'a SegmentCell> for SegmentCellRef<'a> {
    fn from(cell: &'a SegmentCell) -> Self {
        Self {
            candidate_id: cell.candidate_id,
            cell_id: cell.cell_id,
            created_seq: cell.created_seq,
            deleted_seq: cell.deleted_seq,
            descriptor: None,
            payload: &cell.payload,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SegmentCellRecord {
    pub cell: SegmentCell,
    pub descriptor: Option<Vec<u8>>,
}

/// Per-cell segment footer metadata. In ACS3 footer-backed segments this is
/// read from the segment tail, so callers can inspect descriptors and payload
/// locations without decoding every payload.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SegmentDescriptorEntry {
    pub candidate_id: u32,
    pub cell_id: u64,
    pub created_seq: u64,
    pub deleted_seq: Option<u64>,
    pub descriptor: Option<Vec<u8>>,
    pub payload_offset: u64,
    pub payload_len: u64,
    pub payload_crc32c: u32,
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
        let out = codec::encode_segment_refs(cells)?;
        write_atomic(path.as_ref(), &out)?;
        Ok(())
    }

    pub fn write_records(
        path: impl AsRef<Path>,
        records: &[SegmentCellRecord],
    ) -> StorageResult<()> {
        let refs = records
            .iter()
            .map(|record| SegmentCellRef {
                candidate_id: record.cell.candidate_id,
                cell_id: record.cell.cell_id,
                created_seq: record.cell.created_seq,
                deleted_seq: record.cell.deleted_seq,
                descriptor: record.descriptor.clone(),
                payload: &record.cell.payload,
            })
            .collect::<Vec<_>>();
        Self::write_refs(path, &refs)
    }
}

impl SegmentReader {
    pub fn read(path: impl AsRef<Path>) -> StorageResult<Vec<SegmentCell>> {
        let bytes = std::fs::read(path)?;
        decode_segment(&bytes)
    }

    pub fn read_records(path: impl AsRef<Path>) -> StorageResult<Vec<SegmentCellRecord>> {
        let bytes = std::fs::read(path)?;
        codec::decode_segment_records(&bytes)
    }

    pub fn read_lookup(path: impl AsRef<Path>) -> StorageResult<SegmentLookup> {
        SegmentLookup::new(Self::read(path)?)
    }

    /// Read only candidate/cell identity and liveness, skipping payload bytes.
    pub fn read_candidate_entries(
        path: impl AsRef<Path>,
    ) -> StorageResult<Vec<SegmentCandidateEntry>> {
        let bytes = std::fs::read(path)?;
        codec::decode_segment_candidate_entries(&bytes)
    }

    /// Read per-cell descriptor/footer entries without copying payload bytes.
    pub fn read_descriptors(path: impl AsRef<Path>) -> StorageResult<Vec<SegmentDescriptorEntry>> {
        codec::read_segment_descriptors(path.as_ref())
    }

    /// Read a single payload by candidate id.
    ///
    /// Footer-backed ACS3 segments seek directly to the requested payload and
    /// validate that block's CRC. Legacy segments fall back to the compatible
    /// full decode path.
    pub fn read_payload_at(
        path: impl AsRef<Path>,
        candidate_id: u32,
    ) -> StorageResult<Option<Vec<u8>>> {
        codec::read_segment_payload_at(path.as_ref(), candidate_id)
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
    codec::decode_segment_records(bytes).map(|records| {
        records
            .into_iter()
            .map(|record| record.cell)
            .collect::<Vec<_>>()
    })
}

mod codec;

#[cfg(test)]
mod tests;
