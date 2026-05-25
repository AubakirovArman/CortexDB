use std::fs;
use std::io::ErrorKind;

use cortex_core::memtable::MemTableStats;
use cortex_core::CommitSeq;
use cortex_storage::indexes::{BitmapIndex, LexicalIndex};
use cortex_storage::manifest::StorageManifest;
use cortex_storage::segment::SegmentReader;
use cortex_storage::wal::WalReader;

use crate::checkpoint::{bitmap_path, lexical_path, segment_path};
use crate::database::Database;
use crate::error::{EngineError, EngineResult};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StorageStats {
    pub current_seq: CommitSeq,
    pub checkpoint_seq: CommitSeq,
    pub live_segments: usize,
    pub retired_segments: usize,
    pub memtable: MemTableStats,
    pub wal_size_bytes: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StorageValidation {
    pub live_segments_checked: usize,
    pub cells_checked: usize,
    pub wal_records_checked: usize,
    pub wal_safe_truncate_offset: u64,
}

impl Database {
    pub fn storage_stats(&self) -> EngineResult<StorageStats> {
        Ok(StorageStats {
            current_seq: self.current_seq,
            checkpoint_seq: CommitSeq(self.manifest.checkpoint_seq),
            live_segments: self.manifest.live_segments.len(),
            retired_segments: self.manifest.retired_segments.len(),
            memtable: self.memtable.stats(),
            wal_size_bytes: file_len_or_zero(&self.wal_path)?,
        })
    }

    pub fn validate_storage(&self) -> EngineResult<StorageValidation> {
        let manifest = StorageManifest::load(&self.manifest_path)?;
        let mut cells_checked = 0;
        for segment in &manifest.live_segments {
            let segment_file = segment_path(&self.segments_path, segment.id);
            if !segment_file.exists() {
                return Err(EngineError::MissingStorageFile(segment_file));
            }
            let cells = SegmentReader::read(&segment_file)?;
            if cells.len() != segment.cell_count as usize {
                return Err(EngineError::StorageInvariant(format!(
                    "segment {} cell_count mismatch: manifest={} actual={}",
                    segment.id,
                    segment.cell_count,
                    cells.len()
                )));
            }
            if cells.iter().any(|cell| cell.candidate_id == 0) {
                return Err(EngineError::InvalidCandidateId(0));
            }
            BitmapIndex::read(bitmap_path(&self.segments_path, segment.id))?;
            LexicalIndex::read(lexical_path(&self.segments_path, segment.id))?;
            cells_checked += cells.len();
        }
        let wal = WalReader::scan_best_effort_path(&self.wal_path)?;
        Ok(StorageValidation {
            live_segments_checked: manifest.live_segments.len(),
            cells_checked,
            wal_records_checked: wal.records.len(),
            wal_safe_truncate_offset: wal.safe_truncate_offset,
        })
    }
}

fn file_len_or_zero(path: &std::path::Path) -> EngineResult<u64> {
    match fs::metadata(path) {
        Ok(metadata) => Ok(metadata.len()),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(0),
        Err(error) => Err(error.into()),
    }
}
