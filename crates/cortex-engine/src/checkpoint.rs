use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

mod candidates;
mod hnsw;
mod index_merge;
mod paths;
mod vector;

use cortex_core::memtable::MemTable;
use cortex_core::{CellId, CommitSeq};
use cortex_storage::indexes::{BitmapIndex, LexicalIndex};
use cortex_storage::manifest::{ManifestSegment, StorageManifest};
use cortex_storage::segment::{SegmentCell, SegmentReader, SegmentWriter};
use cortex_storage::wal::WalWriter;

use crate::database::{CheckpointStats, Database};
use crate::error::EngineResult;
use crate::query::EngineAqlIndex;
use candidates::{candidate_from_ordinal, segment_cell_count, CandidateAllocator};
use index_merge::{merge_bitmap_index, merge_lexical_index};
pub(crate) use paths::{
    bitmap_path, hnsw_path, lexical_path, manifest_path, segment_path, segments_path, vector_path,
};

pub(crate) struct CheckpointLoad {
    pub manifest: StorageManifest,
    pub memtable: MemTable,
}

pub(crate) struct PersistedIndexState {
    pub bitmap: BitmapIndex,
    pub lexical: LexicalIndex,
    pub candidate_to_cell: BTreeMap<u32, CellId>,
}

impl Database {
    pub fn checkpoint(&mut self) -> EngineResult<CheckpointStats> {
        let base_seq = CommitSeq(self.manifest.checkpoint_seq);
        let candidate_map = self.persisted_candidate_map()?;
        let cells = self.checkpoint_delta_cells(base_seq, &candidate_map)?;
        if cells.is_empty() && self.current_seq == base_seq {
            return Ok(CheckpointStats {
                segment_id: None,
                cells_flushed: 0,
                checkpoint_seq: self.current_seq,
            });
        }

        self.writer.shutdown()?;
        fs::create_dir_all(&self.segments_path)?;
        let segment_id = self.manifest.generation + 1;
        let segment_path = segment_path(&self.segments_path, segment_id);
        SegmentWriter::write(&segment_path, &cells)?;

        let index = EngineAqlIndex::try_from_segment_cells(&cells)?;
        index
            .bitmap_index()
            .write(bitmap_path(&self.segments_path, segment_id))?;
        index
            .lexical_index()
            .write(lexical_path(&self.segments_path, segment_id))?;
        vector::vector_index_for_cells(&cells)
            .write(vector_path(&self.segments_path, segment_id))?;
        hnsw::hnsw_graph_for_cells(&cells).write(hnsw_path(&self.segments_path, segment_id))?;

        self.manifest.checkpoint_segment(ManifestSegment {
            id: segment_id,
            generation: self.manifest.generation + 1,
            checkpoint_seq: self.current_seq.0,
            cell_count: segment_cell_count(cells.len())?,
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

    pub fn compact(&mut self) -> EngineResult<CheckpointStats> {
        let cells = self.full_snapshot_cells()?;
        if cells.is_empty() {
            return Ok(CheckpointStats {
                segment_id: None,
                cells_flushed: 0,
                checkpoint_seq: self.current_seq,
            });
        }

        self.writer.shutdown()?;
        fs::create_dir_all(&self.segments_path)?;
        let segment_id = self.manifest.generation + 1;
        let segment_path = segment_path(&self.segments_path, segment_id);
        SegmentWriter::write(&segment_path, &cells)?;

        let index = EngineAqlIndex::try_from_segment_cells(&cells)?;
        index
            .bitmap_index()
            .write(bitmap_path(&self.segments_path, segment_id))?;
        index
            .lexical_index()
            .write(lexical_path(&self.segments_path, segment_id))?;
        vector::vector_index_for_cells(&cells)
            .write(vector_path(&self.segments_path, segment_id))?;
        hnsw::hnsw_graph_for_cells(&cells).write(hnsw_path(&self.segments_path, segment_id))?;

        self.manifest.compact_to_segment(ManifestSegment {
            id: segment_id,
            generation: self.manifest.generation + 1,
            checkpoint_seq: self.current_seq.0,
            cell_count: segment_cell_count(cells.len())?,
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
        let state = self.persisted_index_state()?;
        Ok((state.bitmap, state.lexical))
    }

    pub(crate) fn persisted_index_state(&self) -> EngineResult<PersistedIndexState> {
        let mut bitmap = BitmapIndex::default();
        let mut lexical = LexicalIndex::default();
        let mut tombstoned = BTreeSet::new();
        let mut candidate_to_cell = BTreeMap::new();
        for segment in &self.manifest.live_segments {
            let cells = SegmentReader::read(segment_path(&self.segments_path, segment.id))?;
            let segment_candidates = cells
                .iter()
                .map(|cell| cell.candidate_id)
                .collect::<BTreeSet<_>>();
            remove_candidates(&mut bitmap, &mut lexical, &segment_candidates);
            for cell in cells {
                if cell.deleted_seq.is_some() {
                    tombstoned.insert(cell.candidate_id);
                    candidate_to_cell.remove(&cell.candidate_id);
                } else {
                    tombstoned.remove(&cell.candidate_id);
                    candidate_to_cell.insert(cell.candidate_id, CellId(cell.cell_id));
                }
            }
            let segment_bitmap = BitmapIndex::read(bitmap_path(&self.segments_path, segment.id))?;
            let segment_lexical =
                LexicalIndex::read(lexical_path(&self.segments_path, segment.id))?;
            merge_bitmap_index(&mut bitmap, segment_bitmap);
            merge_lexical_index(&mut lexical, segment_lexical);
        }
        remove_candidates(&mut bitmap, &mut lexical, &tombstoned);
        Ok(PersistedIndexState {
            bitmap,
            lexical,
            candidate_to_cell,
        })
    }

    fn persisted_candidate_map(&self) -> EngineResult<BTreeMap<CellId, u32>> {
        let mut map = BTreeMap::new();
        for segment in &self.manifest.live_segments {
            for cell in SegmentReader::read(segment_path(&self.segments_path, segment.id))? {
                if cell.deleted_seq.is_some() {
                    map.remove(&CellId(cell.cell_id));
                } else {
                    map.insert(CellId(cell.cell_id), cell.candidate_id);
                }
            }
        }
        Ok(map)
    }

    fn checkpoint_delta_cells(
        &self,
        base_seq: CommitSeq,
        existing: &BTreeMap<CellId, u32>,
    ) -> EngineResult<Vec<SegmentCell>> {
        let txn = self.read_txn();
        let mut allocator = CandidateAllocator::new(existing)?;
        let mut cells = Vec::new();
        for version in self.memtable.visible_cells_created_after(txn, base_seq) {
            cells.push(SegmentCell {
                candidate_id: allocator.candidate_for(version.cell_id)?,
                cell_id: version.cell_id.0,
                created_seq: version.created_seq.0,
                deleted_seq: None,
                payload: version.payload,
            });
        }
        for (cell_id, deleted) in self.memtable.tombstones_after(base_seq) {
            cells.push(SegmentCell {
                candidate_id: allocator.candidate_for(cell_id)?,
                cell_id: cell_id.0,
                created_seq: 0,
                deleted_seq: Some(deleted.0),
                payload: Vec::new(),
            });
        }
        cells.sort_by_key(|cell| {
            (
                cell.deleted_seq.unwrap_or(cell.created_seq),
                cell.cell_id,
                cell.created_seq,
            )
        });
        Ok(cells)
    }

    fn full_snapshot_cells(&self) -> EngineResult<Vec<SegmentCell>> {
        self.snapshot_versions()
            .into_iter()
            .enumerate()
            .map(|version| {
                Ok(SegmentCell {
                    candidate_id: candidate_from_ordinal(version.0)?,
                    cell_id: version.1.cell_id.0,
                    created_seq: version.1.created_seq.0,
                    deleted_seq: None,
                    payload: version.1.payload,
                })
            })
            .collect()
    }
}

fn remove_candidates(
    bitmap: &mut BitmapIndex,
    lexical: &mut LexicalIndex,
    candidates: &BTreeSet<u32>,
) {
    for values in bitmap.bitmaps.values_mut() {
        values.retain(|candidate| !candidates.contains(candidate));
    }
    bitmap.bitmaps.retain(|_, values| !values.is_empty());
    for values in lexical.terms.values_mut() {
        values.retain(|candidate| !candidates.contains(candidate));
    }
    lexical.terms.retain(|_, values| !values.is_empty());
    lexical
        .doc_lengths
        .retain(|candidate, _| !candidates.contains(candidate));
    for values in lexical.term_frequencies.values_mut() {
        values.retain(|candidate, _| !candidates.contains(candidate));
    }
    lexical
        .term_frequencies
        .retain(|_, values| !values.is_empty());
}

pub(crate) fn load_checkpoint(root: &Path) -> EngineResult<CheckpointLoad> {
    let manifest = StorageManifest::load(manifest_path(root))?;
    let mut memtable = MemTable::default();
    for segment in &manifest.live_segments {
        let cells = SegmentReader::read(segment_path(&segments_path(root), segment.id))?;
        for cell in cells {
            if let Some(deleted) = cell.deleted_seq {
                memtable.record_tombstone(CellId(cell.cell_id), CommitSeq(deleted));
            } else {
                memtable.put_cell(
                    CellId(cell.cell_id),
                    CommitSeq(cell.created_seq),
                    cell.payload,
                );
            }
        }
    }
    Ok(CheckpointLoad { manifest, memtable })
}
