use std::fs;

use cortex_core::memtable::MemTable;
use cortex_core::{CellDescriptor, CellId, CommitSeq};
use cortex_storage::manifest::ManifestSegment;
use cortex_storage::segment::{SegmentCellRef, SegmentWriter};
use cortex_storage::wal::WalWriter;

use crate::checkpoint::hnsw::hnsw_graph_for_cell_refs_with_config;
use crate::checkpoint::manifest_hnsw_profile;
use crate::checkpoint::vector::vector_index_for_cell_refs;
use crate::checkpoint::{bitmap_path, hnsw_path, lexical_path, segment_path, vector_path};
use crate::database::{CheckpointStats, Database};
use crate::error::{EngineError, EngineResult};
use crate::options::EngineFeature;
use crate::query::EngineAqlIndex;

use super::{SnapshotCell, SnapshotSegment};

impl Database {
    pub fn replication_snapshot_segment(&self) -> EngineResult<SnapshotSegment> {
        self.require_feature(EngineFeature::ExperimentalReplication)?;
        let cells = self
            .full_snapshot_cell_refs()?
            .into_iter()
            .map(SnapshotCell::from_segment_cell_ref)
            .collect();
        Ok(SnapshotSegment {
            checkpoint_seq: self.current_seq,
            cells,
        })
    }

    pub fn install_snapshot_segment(
        &mut self,
        snapshot: SnapshotSegment,
    ) -> EngineResult<CheckpointStats> {
        self.require_feature(EngineFeature::ExperimentalReplication)?;
        validate_snapshot_cells(&snapshot.cells)?;
        self.writer.shutdown()?;
        fs::create_dir_all(&self.segments_path)?;
        let segment_id = self.manifest.generation + 1;
        let cell_refs = snapshot_cell_refs(&snapshot);
        SegmentWriter::write_refs(segment_path(&self.segments_path, segment_id), &cell_refs)?;

        let index = EngineAqlIndex::try_from_segment_cell_refs(&cell_refs)?;
        index
            .bitmap_index()
            .write(bitmap_path(&self.segments_path, segment_id))?;
        index
            .lexical_index()
            .write(lexical_path(&self.segments_path, segment_id))?;
        vector_index_for_cell_refs(&cell_refs)
            .write(vector_path(&self.segments_path, segment_id))?;
        if self.feature_flags.experimental_hnsw {
            hnsw_graph_for_cell_refs_with_config(&cell_refs, self.hnsw_build_config)?
                .write(hnsw_path(&self.segments_path, segment_id))?;
        }

        self.manifest.compact_to_segment(ManifestSegment {
            id: segment_id,
            generation: self.manifest.generation + 1,
            checkpoint_seq: snapshot.checkpoint_seq.0,
            cell_count: cell_count(snapshot.cells.len())?,
        });
        self.manifest.hnsw_profile = self
            .feature_flags
            .experimental_hnsw
            .then(|| manifest_hnsw_profile(self.hnsw_build_config))
            .transpose()?;
        self.manifest.store(&self.manifest_path)?;
        crate::database_files::truncate_wal_tail(&self.wal_path, 0)?;
        self.memtable = memtable_from_snapshot(&snapshot);
        self.current_seq = snapshot.checkpoint_seq;
        self.aql_delta_index.clear();
        self.rebuild_derived_stores_from_memtable();
        if let Ok(mut cache) = self.persisted_index_cache.lock() {
            *cache = None;
        }
        self.writer = WalWriter::start(&self.wal_path, self.durability_mode)?;
        Ok(CheckpointStats {
            segment_id: Some(segment_id),
            cells_flushed: snapshot.cells.len(),
            checkpoint_seq: snapshot.checkpoint_seq,
        })
    }
}

fn memtable_from_snapshot(snapshot: &SnapshotSegment) -> MemTable {
    let mut memtable = MemTable::default();
    for cell in &snapshot.cells {
        if let Some(deleted) = cell.deleted_seq {
            memtable.record_tombstone(CellId(cell.cell_id), CommitSeq(deleted));
        } else {
            let descriptor = cell
                .descriptor
                .as_deref()
                .and_then(CellDescriptor::decode_section_v1)
                .unwrap_or_else(|| CellDescriptor::from_payload_lossy(&cell.payload));
            memtable.put_cell_with_descriptor(
                CellId(cell.cell_id),
                CommitSeq(cell.created_seq),
                cell.payload.clone(),
                descriptor,
            );
        }
    }
    memtable
}

fn validate_snapshot_cells(cells: &[SnapshotCell]) -> EngineResult<()> {
    if cells.iter().any(|cell| cell.candidate_id == 0) {
        return Err(EngineError::InvalidCandidateId(0));
    }
    Ok(())
}

fn snapshot_cell_refs(snapshot: &SnapshotSegment) -> Vec<SegmentCellRef<'_>> {
    snapshot
        .cells
        .iter()
        .map(SnapshotCell::segment_cell_ref)
        .collect()
}

fn cell_count(len: usize) -> EngineResult<u32> {
    u32::try_from(len).map_err(|_| EngineError::CandidateIdOverflow)
}
