use std::fs;

use cortex_core::memtable::MemTable;
use cortex_core::{CellId, CommitSeq};
use cortex_storage::manifest::ManifestSegment;
use cortex_storage::segment::{SegmentCell, SegmentWriter};
use cortex_storage::wal::WalWriter;

use crate::checkpoint::hnsw::hnsw_graph_for_cells;
use crate::checkpoint::vector::vector_index_for_cells;
use crate::checkpoint::{bitmap_path, hnsw_path, lexical_path, segment_path, vector_path};
use crate::database::{CheckpointStats, Database};
use crate::error::{EngineError, EngineResult};
use crate::query::EngineAqlIndex;

use super::SnapshotSegment;

impl Database {
    pub fn install_snapshot_segment(
        &mut self,
        snapshot: SnapshotSegment,
    ) -> EngineResult<CheckpointStats> {
        validate_snapshot_cells(&snapshot.cells)?;
        self.writer.shutdown()?;
        fs::create_dir_all(&self.segments_path)?;
        let segment_id = self.manifest.generation + 1;
        SegmentWriter::write(
            segment_path(&self.segments_path, segment_id),
            &snapshot.cells,
        )?;

        let index = EngineAqlIndex::try_from_segment_cells(&snapshot.cells)?;
        index
            .bitmap_index()
            .write(bitmap_path(&self.segments_path, segment_id))?;
        index
            .lexical_index()
            .write(lexical_path(&self.segments_path, segment_id))?;
        vector_index_for_cells(&snapshot.cells)
            .write(vector_path(&self.segments_path, segment_id))?;
        hnsw_graph_for_cells(&snapshot.cells)?.write(hnsw_path(&self.segments_path, segment_id))?;

        self.manifest.compact_to_segment(ManifestSegment {
            id: segment_id,
            generation: self.manifest.generation + 1,
            checkpoint_seq: snapshot.checkpoint_seq.0,
            cell_count: cell_count(snapshot.cells.len())?,
        });
        self.manifest.store(&self.manifest_path)?;
        crate::database::truncate_wal_tail(&self.wal_path, 0)?;
        self.memtable = memtable_from_snapshot(&snapshot);
        self.current_seq = snapshot.checkpoint_seq;
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
            memtable.put_cell(
                CellId(cell.cell_id),
                CommitSeq(cell.created_seq),
                cell.payload.clone(),
            );
        }
    }
    memtable
}

fn validate_snapshot_cells(cells: &[SegmentCell]) -> EngineResult<()> {
    if cells.iter().any(|cell| cell.candidate_id == 0) {
        return Err(EngineError::InvalidCandidateId(0));
    }
    Ok(())
}

fn cell_count(len: usize) -> EngineResult<u32> {
    u32::try_from(len).map_err(|_| EngineError::CandidateIdOverflow)
}
