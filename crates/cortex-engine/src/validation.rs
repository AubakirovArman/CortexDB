use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::ErrorKind;

use cortex_core::memtable::MemTableStats;
use cortex_core::CommitSeq;
use cortex_storage::indexes::{BitmapIndex, LexicalIndex};
use cortex_storage::manifest::StorageManifest;
use cortex_storage::segment::SegmentReader;
use cortex_storage::vectors::VectorIndex;
use cortex_storage::wal::{WalReader, WalWriterMetrics};

use crate::checkpoint::{bitmap_path, lexical_path, segment_path, vector_path};
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
    pub wal_writer: WalWriterMetrics,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StorageValidation {
    pub live_segments_checked: usize,
    pub cells_checked: usize,
    pub wal_records_checked: usize,
    pub wal_safe_truncate_offset: u64,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StorageValidationReport {
    pub manifest_ok: bool,
    pub wal_ok: bool,
    pub live_segments_checked: usize,
    pub bitmap_indexes_checked: usize,
    pub lexical_indexes_checked: usize,
    pub vector_indexes_checked: usize,
    pub cells_checked: usize,
    pub wal_records_checked: usize,
    pub wal_safe_truncate_offset: u64,
    pub errors: Vec<String>,
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
            wal_writer: self.writer.metrics()?,
        })
    }

    pub fn validate_storage(&self) -> EngineResult<StorageValidation> {
        let report = self.validate_storage_report();
        if !report.errors.is_empty() {
            return Err(EngineError::StorageInvariant(report.errors.join("; ")));
        }
        Ok(StorageValidation {
            live_segments_checked: report.live_segments_checked,
            cells_checked: report.cells_checked,
            wal_records_checked: report.wal_records_checked,
            wal_safe_truncate_offset: report.wal_safe_truncate_offset,
        })
    }

    pub fn validate_storage_report(&self) -> StorageValidationReport {
        let mut report = StorageValidationReport::default();
        let manifest = match StorageManifest::load(&self.manifest_path) {
            Ok(manifest) => {
                report.manifest_ok = true;
                manifest
            }
            Err(error) => {
                report.errors.push(format!("manifest: {error}"));
                check_wal(&self.wal_path, &mut report);
                return report;
            }
        };
        let mut cells_checked = 0;
        let mut live_ids = BTreeSet::new();
        let mut retired_ids = BTreeSet::new();
        let mut candidates = BTreeMap::new();
        for segment in &manifest.live_segments {
            if !live_ids.insert(segment.id) {
                report
                    .errors
                    .push(format!("duplicate live segment id: {}", segment.id));
            }
            if manifest.checkpoint_seq < segment.checkpoint_seq {
                report.errors.push(format!(
                    "manifest checkpoint_seq {} is behind segment {} checkpoint_seq {}",
                    manifest.checkpoint_seq, segment.id, segment.checkpoint_seq
                ));
            }
            let segment_file = segment_path(&self.segments_path, segment.id);
            if !segment_file.exists() {
                report
                    .errors
                    .push(format!("missing storage file: {}", segment_file.display()));
                continue;
            }
            let cells = match SegmentReader::read(&segment_file) {
                Ok(cells) => cells,
                Err(error) => {
                    report
                        .errors
                        .push(format!("segment {}: {error}", segment.id));
                    continue;
                }
            };
            report.live_segments_checked += 1;
            if cells.len() != segment.cell_count as usize {
                report.errors.push(format!(
                    "segment {} cell_count mismatch: manifest={} actual={}",
                    segment.id,
                    segment.cell_count,
                    cells.len()
                ));
            }
            if cells.iter().any(|cell| cell.candidate_id == 0) {
                report.errors.push("invalid candidate id: 0".to_owned());
            }
            for cell in &cells {
                if let Some(previous) = candidates.insert(cell.candidate_id, cell.cell_id) {
                    if previous != cell.cell_id {
                        report.errors.push(format!(
                            "candidate {} maps to multiple cells",
                            cell.candidate_id
                        ));
                    }
                }
            }
            match BitmapIndex::read(bitmap_path(&self.segments_path, segment.id)) {
                Ok(_) => report.bitmap_indexes_checked += 1,
                Err(error) => report
                    .errors
                    .push(format!("bitmap index {}: {error}", segment.id)),
            }
            match LexicalIndex::read(lexical_path(&self.segments_path, segment.id)) {
                Ok(_) => report.lexical_indexes_checked += 1,
                Err(error) => report
                    .errors
                    .push(format!("lexical index {}: {error}", segment.id)),
            }
            match VectorIndex::read(vector_path(&self.segments_path, segment.id)) {
                Ok(_) => report.vector_indexes_checked += 1,
                Err(error) => report
                    .errors
                    .push(format!("vector index {}: {error}", segment.id)),
            }
            cells_checked += cells.len();
        }
        for segment in &manifest.retired_segments {
            if !retired_ids.insert(segment.id) || live_ids.contains(&segment.id) {
                report.errors.push(format!(
                    "retired segment {} conflicts with manifest references",
                    segment.id
                ));
            }
        }
        report.cells_checked = cells_checked;
        check_wal(&self.wal_path, &mut report);
        report
    }
}

fn check_wal(path: &std::path::Path, report: &mut StorageValidationReport) {
    match WalReader::scan_best_effort_path(path) {
        Ok(wal) => {
            report.wal_ok = true;
            report.wal_records_checked = wal.records.len();
            report.wal_safe_truncate_offset = wal.safe_truncate_offset;
        }
        Err(error) => report.errors.push(format!("wal: {error}")),
    }
}

fn file_len_or_zero(path: &std::path::Path) -> EngineResult<u64> {
    match fs::metadata(path) {
        Ok(metadata) => Ok(metadata.len()),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(0),
        Err(error) => Err(error.into()),
    }
}
