use std::fs;
use std::path::{Path, PathBuf};

use cortex_storage::hnsw::HnswGraphIndex;
use cortex_storage::indexes::{BitmapIndex, LexicalIndex};
use cortex_storage::segment::SegmentReader;
use cortex_storage::vectors::VectorIndex;
use cortex_storage::wal::WalReader;

use crate::backup::{
    reject_existing_target, reject_target_inside_source, should_skip_backup_entry,
};
use crate::checkpoint::{
    bitmap_path, hnsw_path, lexical_path, load_checkpoint, segment_path, segments_path, vector_path,
};
use crate::error::{EngineError, EngineResult};
use crate::validation::StorageValidation;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RestoreDryRunReport {
    pub restore_path: PathBuf,
    pub files_checked: usize,
    pub bytes_checked: u64,
    pub backup_validation: StorageValidation,
    pub version_compatible: bool,
}

pub(super) fn restore_from_backup_dry_run(
    backup_path: &Path,
    target_path: &Path,
) -> EngineResult<RestoreDryRunReport> {
    let backup_path = backup_path.canonicalize()?;
    reject_target_inside_source(&backup_path, target_path)?;
    reject_existing_target(target_path)?;

    let checked = inspect_database_dir(&backup_path)?;
    let backup_validation = validate_backup_read_only(&backup_path)?;
    Ok(RestoreDryRunReport {
        restore_path: target_path.to_owned(),
        files_checked: checked.files_checked,
        bytes_checked: checked.bytes_checked,
        backup_validation,
        version_compatible: true,
    })
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct InspectReport {
    files_checked: usize,
    bytes_checked: u64,
}

fn inspect_database_dir(path: &Path) -> EngineResult<InspectReport> {
    let mut report = InspectReport::default();
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if should_skip_backup_entry(&name) {
            continue;
        }
        let file_type = entry.file_type()?;
        let entry_path = entry.path();
        if file_type.is_dir() {
            let child = inspect_database_dir(&entry_path)?;
            report.files_checked += child.files_checked;
            report.bytes_checked += child.bytes_checked;
        } else if file_type.is_file() {
            report.files_checked += 1;
            report.bytes_checked += entry.metadata()?.len();
        } else {
            return Err(EngineError::StorageInvariant(format!(
                "backup refuses non-regular file: {}",
                entry_path.display()
            )));
        }
    }
    Ok(report)
}

fn validate_backup_read_only(root: &Path) -> EngineResult<StorageValidation> {
    let checkpoint = load_checkpoint(root)?;
    let segments = segments_path(root);
    let mut cells_checked = 0;
    for segment in &checkpoint.manifest.live_segments {
        let cells = SegmentReader::read(segment_path(&segments, segment.id))?;
        if cells.len() != segment.cell_count as usize {
            return Err(EngineError::StorageInvariant(format!(
                "segment {} cell_count mismatch: manifest={} actual={}",
                segment.id,
                segment.cell_count,
                cells.len()
            )));
        }
        BitmapIndex::read(bitmap_path(&segments, segment.id))?;
        LexicalIndex::read(lexical_path(&segments, segment.id))?;
        VectorIndex::read(vector_path(&segments, segment.id))?;
        HnswGraphIndex::read(hnsw_path(&segments, segment.id))?;
        cells_checked += cells.len();
    }

    let wal_path = root.join("db.aclog");
    if !wal_path.exists() {
        return Err(EngineError::MissingStorageFile(wal_path));
    }
    let wal = WalReader::scan_best_effort_path(&wal_path)?;
    Ok(StorageValidation {
        live_segments_checked: checkpoint.manifest.live_segments.len(),
        cells_checked,
        wal_records_checked: wal.records.len(),
        wal_safe_truncate_offset: wal.safe_truncate_offset,
    })
}
