use std::fs;
use std::path::{Path, PathBuf};

use cortex_core::memtable::MemTable;
use cortex_core::{CellId, CommitSeq};
use cortex_storage::indexes::{BitmapIndex, LexicalIndex};
use cortex_storage::manifest::{ManifestSegment, StorageManifest};
use cortex_storage::segment::{SegmentCell, SegmentReader, SegmentWriter};
use cortex_storage::wal::WalWriter;

use crate::database::{CheckpointStats, Database};
use crate::error::EngineResult;
use crate::query::EngineAqlIndex;

pub(crate) struct CheckpointLoad {
    pub manifest: StorageManifest,
    pub memtable: MemTable,
}

impl Database {
    pub fn checkpoint(&mut self) -> EngineResult<CheckpointStats> {
        let versions = self.snapshot_versions();
        if versions.is_empty() {
            return Ok(CheckpointStats {
                segment_id: None,
                cells_flushed: 0,
                checkpoint_seq: self.current_seq,
            });
        }

        self.writer.shutdown()?;
        fs::create_dir_all(&self.segments_path)?;
        let segment_id = self.manifest.generation + 1;
        let cells: Vec<_> = versions
            .into_iter()
            .map(|version| SegmentCell {
                cell_id: version.cell_id.0,
                created_seq: version.created_seq.0,
                payload: version.payload,
            })
            .collect();
        let segment_path = segment_path(&self.segments_path, segment_id);
        SegmentWriter::write(&segment_path, &cells)?;

        let index = EngineAqlIndex::from_segment_cells(&cells);
        index
            .bitmap_index()
            .write(bitmap_path(&self.segments_path, segment_id))?;
        index
            .lexical_index()
            .write(lexical_path(&self.segments_path, segment_id))?;

        self.manifest.checkpoint_segment(ManifestSegment {
            id: segment_id,
            generation: self.manifest.generation + 1,
            checkpoint_seq: self.current_seq.0,
            cell_count: cells.len() as u32,
        });
        self.manifest.store(&self.manifest_path)?;
        super::database::truncate_wal_tail(&self.wal_path, 0)?;
        self.writer = WalWriter::start(&self.wal_path, self.durability_mode)?;
        Ok(CheckpointStats {
            segment_id: Some(segment_id),
            cells_flushed: cells.len(),
            checkpoint_seq: self.current_seq,
        })
    }

    pub fn persisted_indexes(&self) -> EngineResult<(BitmapIndex, LexicalIndex)> {
        let mut bitmap = BitmapIndex::default();
        let mut lexical = LexicalIndex::default();
        for segment in &self.manifest.live_segments {
            let segment_bitmap = BitmapIndex::read(bitmap_path(&self.segments_path, segment.id))?;
            let segment_lexical =
                LexicalIndex::read(lexical_path(&self.segments_path, segment.id))?;
            bitmap.bitmaps.extend(segment_bitmap.bitmaps);
            lexical.terms.extend(segment_lexical.terms);
        }
        Ok((bitmap, lexical))
    }
}

pub(crate) fn load_checkpoint(root: &Path) -> EngineResult<CheckpointLoad> {
    let manifest = StorageManifest::load(manifest_path(root))?;
    let mut memtable = MemTable::default();
    for segment in &manifest.live_segments {
        let cells = SegmentReader::read(segment_path(&segments_path(root), segment.id))?;
        for cell in cells {
            memtable.put_cell(
                CellId(cell.cell_id),
                CommitSeq(cell.created_seq),
                cell.payload,
            );
        }
    }
    Ok(CheckpointLoad { manifest, memtable })
}

pub(crate) fn manifest_path(root: &Path) -> PathBuf {
    root.join("manifest.acm")
}

pub(crate) fn segments_path(root: &Path) -> PathBuf {
    root.join("segments")
}

fn segment_path(root: &Path, id: u64) -> PathBuf {
    root.join(format!("segment-{id}.acs"))
}

fn bitmap_path(root: &Path, id: u64) -> PathBuf {
    root.join(format!("segment-{id}.acb"))
}

fn lexical_path(root: &Path, id: u64) -> PathBuf {
    root.join(format!("segment-{id}.aci"))
}
