use std::collections::BTreeMap;
use std::fs;

use cortex_core::{CellId, CommitSeq};
use cortex_storage::manifest::ManifestSegment;
use cortex_storage::segment::{SegmentCellRef, SegmentWriter};

use crate::database::{CheckpointStats, Database};
use crate::error::{EngineError, EngineResult};
use crate::query::EngineAqlIndex;

use super::candidates::{candidate_from_ordinal, segment_cell_count, CandidateAllocator};
use super::profiles::ensure_checkpoint_profiles;
use super::{
    bitmap_path, hnsw, hnsw_path, lexical_path, manifest_hnsw_profile, segment_path, vector,
    vector_path,
};

impl Database {
    pub fn checkpoint(&mut self) -> EngineResult<CheckpointStats> {
        let base_seq = CommitSeq(self.manifest.checkpoint_seq);
        let candidate_map = self.persisted_candidate_map()?;
        let txn = self.read_txn();
        let tombstones = self.memtable.tombstones_after(base_seq);
        let live_delta_count = self
            .memtable
            .visible_created_after_iter(txn, base_seq)
            .count();
        if live_delta_count == 0 && tombstones.is_empty() && self.current_seq == base_seq {
            return Ok(CheckpointStats {
                segment_id: None,
                cells_flushed: 0,
                checkpoint_seq: self.current_seq,
            });
        }
        let hnsw_profile = self
            .feature_flags
            .experimental_hnsw
            .then(|| manifest_hnsw_profile(self.hnsw_build_config))
            .transpose()?;
        let vector_profile = {
            let cells = self.checkpoint_delta_cell_refs(base_seq, &candidate_map, &tombstones)?;
            vector::vector_profile_for_cell_refs(&cells, self.hnsw_build_config)
        }?;
        ensure_checkpoint_profiles(&self.manifest, hnsw_profile, vector_profile)?;

        let archived_wal = self.writer.rotate().map_err(|e| {
            EngineError::Storage(cortex_storage::StorageError::Io(std::io::Error::other(
                format!("failed to rotate WAL before checkpoint: {e}"),
            )))
        })?;
        fs::create_dir_all(&self.segments_path)?;
        let segment_id = self.manifest.generation + 1;
        let segment_path = segment_path(&self.segments_path, segment_id);
        let cells_flushed = {
            let cells = self.checkpoint_delta_cell_refs(base_seq, &candidate_map, &tombstones)?;
            SegmentWriter::write_refs(&segment_path, &cells)?;

            let index = EngineAqlIndex::try_from_segment_cell_refs(&cells)?;
            index
                .bitmap_index()
                .write(bitmap_path(&self.segments_path, segment_id))?;
            index
                .lexical_index()
                .write(lexical_path(&self.segments_path, segment_id))?;
            vector::vector_index_for_cell_refs(&cells)
                .write(vector_path(&self.segments_path, segment_id))?;
            if self.feature_flags.experimental_hnsw {
                hnsw::hnsw_graph_for_cell_refs_with_config(&cells, self.hnsw_build_config)?
                    .write(hnsw_path(&self.segments_path, segment_id))?;
            }
            cells.len()
        };
        self.publish_checkpoint_corpus_synonym_dictionary()?;

        self.manifest.checkpoint_segment(ManifestSegment {
            id: segment_id,
            generation: self.manifest.generation + 1,
            checkpoint_seq: self.current_seq.0,
            cell_count: segment_cell_count(cells_flushed)?,
        });
        self.manifest.hnsw_profile = hnsw_profile;
        if let Some(profile) = vector_profile {
            self.manifest.vector_profile = Some(profile);
        }
        self.manifest.store(&self.manifest_path)?;
        let _ = std::fs::remove_file(&archived_wal);
        self.memtable.gc_versions_before(self.gc_horizon());
        self.aql_delta_index.clear();
        Ok(CheckpointStats {
            segment_id: Some(segment_id),
            cells_flushed,
            checkpoint_seq: self.current_seq,
        })
    }

    pub fn compact(&mut self) -> EngineResult<CheckpointStats> {
        let hnsw_profile = self
            .feature_flags
            .experimental_hnsw
            .then(|| manifest_hnsw_profile(self.hnsw_build_config))
            .transpose()?;
        if self.memtable.visible_iter(self.read_txn()).next().is_none() {
            self.publish_checkpoint_corpus_synonym_dictionary()?;
            return Ok(CheckpointStats {
                segment_id: None,
                cells_flushed: 0,
                checkpoint_seq: self.current_seq,
            });
        }
        let vector_profile = {
            let cells = self.full_snapshot_cell_refs()?;
            vector::vector_profile_for_cell_refs(&cells, self.hnsw_build_config)
        }?;

        let archived_wal = self.writer.rotate().map_err(|e| {
            EngineError::Storage(cortex_storage::StorageError::Io(std::io::Error::other(
                format!("failed to rotate WAL before compact: {e}"),
            )))
        })?;
        fs::create_dir_all(&self.segments_path)?;
        let segment_id = self.manifest.generation + 1;
        let segment_path = segment_path(&self.segments_path, segment_id);
        let cells_flushed = {
            let cells = self.full_snapshot_cell_refs()?;
            SegmentWriter::write_refs(&segment_path, &cells)?;

            let index = EngineAqlIndex::try_from_segment_cell_refs(&cells)?;
            index
                .bitmap_index()
                .write(bitmap_path(&self.segments_path, segment_id))?;
            index
                .lexical_index()
                .write(lexical_path(&self.segments_path, segment_id))?;
            vector::vector_index_for_cell_refs(&cells)
                .write(vector_path(&self.segments_path, segment_id))?;
            if self.feature_flags.experimental_hnsw {
                hnsw::hnsw_graph_for_cell_refs_with_config(&cells, self.hnsw_build_config)?
                    .write(hnsw_path(&self.segments_path, segment_id))?;
            }
            cells.len()
        };
        self.publish_checkpoint_corpus_synonym_dictionary()?;

        self.manifest.compact_to_segment(ManifestSegment {
            id: segment_id,
            generation: self.manifest.generation + 1,
            checkpoint_seq: self.current_seq.0,
            cell_count: segment_cell_count(cells_flushed)?,
        });
        self.manifest.hnsw_profile = hnsw_profile;
        self.manifest.vector_profile = vector_profile;
        self.manifest.store(&self.manifest_path)?;
        let _ = std::fs::remove_file(&archived_wal);
        self.memtable.gc_versions_before(self.gc_horizon());
        self.aql_delta_index.clear();
        Ok(CheckpointStats {
            segment_id: Some(segment_id),
            cells_flushed,
            checkpoint_seq: self.current_seq,
        })
    }

    fn persisted_candidate_map(&self) -> EngineResult<BTreeMap<CellId, u32>> {
        let mut map = BTreeMap::new();
        for segment in &self.manifest.live_segments {
            for cell in cortex_storage::segment::SegmentReader::read(segment_path(
                &self.segments_path,
                segment.id,
            ))? {
                if cell.deleted_seq.is_some() {
                    map.remove(&CellId(cell.cell_id));
                } else {
                    map.insert(CellId(cell.cell_id), cell.candidate_id);
                }
            }
        }
        Ok(map)
    }

    fn checkpoint_delta_cell_refs<'a>(
        &'a self,
        base_seq: CommitSeq,
        existing: &BTreeMap<CellId, u32>,
        tombstones: &[(CellId, CommitSeq)],
    ) -> EngineResult<Vec<SegmentCellRef<'a>>> {
        let txn = self.read_txn();
        let mut allocator = CandidateAllocator::new(existing)?;
        let mut cells = Vec::new();
        for version in self.memtable.visible_created_after_iter(txn, base_seq) {
            cells.push(SegmentCellRef {
                candidate_id: allocator.candidate_for(version.cell_id)?,
                cell_id: version.cell_id.0,
                created_seq: version.created_seq.0,
                deleted_seq: None,
                descriptor: Some(version.descriptor.encode_section_v1()),
                payload: version.payload.as_slice(),
            });
        }
        for (cell_id, deleted) in tombstones {
            cells.push(SegmentCellRef {
                candidate_id: allocator.candidate_for(*cell_id)?,
                cell_id: cell_id.0,
                created_seq: 0,
                deleted_seq: Some(deleted.0),
                descriptor: None,
                payload: &[],
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

    pub(crate) fn full_snapshot_cell_refs(&self) -> EngineResult<Vec<SegmentCellRef<'_>>> {
        self.memtable
            .visible_iter(self.read_txn())
            .enumerate()
            .map(|version| {
                Ok(SegmentCellRef {
                    candidate_id: candidate_from_ordinal(version.0)?,
                    cell_id: version.1.cell_id.0,
                    created_seq: version.1.created_seq.0,
                    deleted_seq: None,
                    descriptor: Some(version.1.descriptor.encode_section_v1()),
                    payload: version.1.payload.as_slice(),
                })
            })
            .collect()
    }
}
