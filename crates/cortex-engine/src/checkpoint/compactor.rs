use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::time::Instant;

use cortex_core::{CellDescriptor, CellId};
use cortex_storage::manifest::ManifestSegment;
use cortex_storage::segment::{SegmentCellRecord, SegmentCellRef, SegmentReader, SegmentWriter};

use crate::bundle::SegmentBundle;
use crate::database::Database;
use crate::error::EngineResult;
use crate::query::EngineAqlIndex;

use super::candidates::{candidate_from_ordinal, segment_cell_count};
use super::vector::vector_profile_for_cell_refs;
use super::{
    bitmap_path, ensure_checkpoint_profiles, hnsw_path, lexical_path, manifest_hnsw_profile,
    segment_path, vector_path,
};

/// Statistics returned by an incremental compaction pass.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CompactionStats {
    /// Live segments before compaction.
    pub segments_before: usize,
    /// Live segments after compaction.
    pub segments_after: usize,
    /// Number of live cells written into the new segment.
    pub cells_compacted: usize,
    /// Total input bundle bytes of the merged segments.
    pub input_bytes: u64,
    /// Total output bundle bytes of the new segment.
    pub output_bytes: u64,
    /// Wall-clock time of the compaction in milliseconds.
    pub duration_ms: u64,
}

/// Reason why compaction was triggered.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompactionDecision {
    /// Pressure and segment count are below configured thresholds.
    NotNeeded,
    /// Retired/total segment pressure crossed the threshold.
    Pressure { pressure_q16: u32 },
    /// Number of live segments crossed the threshold.
    SegmentCount { count: usize },
}

impl Database {
    /// Decide whether incremental compaction is warranted based on the current
    /// storage stats and the configured compaction policy.
    pub fn compaction_decision(&self) -> EngineResult<CompactionDecision> {
        let stats = self.storage_stats()?;
        if stats.live_segments >= self.compaction_policy.trigger_segment_count {
            return Ok(CompactionDecision::SegmentCount {
                count: stats.live_segments,
            });
        }
        if stats.compaction_pressure_q16 >= self.compaction_policy.trigger_pressure_q16 {
            return Ok(CompactionDecision::Pressure {
                pressure_q16: stats.compaction_pressure_q16,
            });
        }
        Ok(CompactionDecision::NotNeeded)
    }

    /// Run incremental compaction only if the current state crosses a trigger
    /// threshold. Returns `None` when no work was performed.
    pub fn maybe_incremental_compact(&mut self) -> EngineResult<Option<CompactionStats>> {
        match self.compaction_decision()? {
            CompactionDecision::NotNeeded => Ok(None),
            _ => self.incremental_compact().map(Some),
        }
    }

    /// Merge a selected subset of live segments into one new segment, replace
    /// the input segments in the manifest, and move the old segment files to
    /// the retired list. The checkpoint sequence is left unchanged: this
    /// operation compacts existing persisted state without advancing the WAL
    /// truncation horizon.
    ///
    /// The merge is performed under the caller's exclusive write lock, so the
    /// memtable snapshot used for deduplication is stable for the duration of
    /// the pass.
    pub fn incremental_compact(&mut self) -> EngineResult<CompactionStats> {
        self.manifest.compaction_metadata.triggered += 1;
        let selected_indices = self.select_segments_for_compaction();
        if selected_indices.len() < 2 {
            return Ok(CompactionStats::default());
        }

        let start = Instant::now();
        let read_txn = self.read_txn();
        let deleted_cells: BTreeSet<CellId> =
            self.memtable.deleted_cell_ids().into_iter().collect();

        let mut merged: BTreeMap<CellId, SegmentCellRecord> = BTreeMap::new();
        let mut input_bytes: u64 = 0;
        let segments_before = self.manifest.live_segments.len();

        for idx in &selected_indices {
            let segment = &self.manifest.live_segments[*idx];
            let path = segment_path(&self.segments_path, segment.id);
            let records = SegmentReader::read_records(&path)?;
            for record in records {
                let cell_id = CellId(record.cell.cell_id);

                // A newer visible version in the memtable supersedes the
                // segment record. The memtable may still retain the latest
                // visible version at or below the checkpoint sequence, so we
                // compare sequence numbers rather than treating any visible
                // memtable entry as newer.
                if let Some(version) = self.memtable.read(read_txn, cell_id) {
                    if version.created_seq.0 > record.cell.created_seq {
                        continue;
                    }
                }
                // A tombstone in the memtable deletes the cell entirely.
                if deleted_cells.contains(&cell_id) {
                    continue;
                }

                if record.cell.deleted_seq.is_some() {
                    merged.remove(&cell_id);
                    continue;
                }

                let keep = match merged.get(&cell_id) {
                    Some(existing) => record.cell.created_seq > existing.cell.created_seq,
                    None => true,
                };
                if keep {
                    merged.insert(cell_id, record);
                }
            }
            input_bytes += self.segment_bundle_bytes(segment.id);
        }

        let hnsw_profile = self
            .feature_flags
            .experimental_hnsw
            .then(|| manifest_hnsw_profile(self.hnsw_build_config))
            .transpose()?;

        let cells = build_cell_refs(&merged)?;
        let vector_profile = vector_profile_for_cell_refs(&cells, self.hnsw_build_config)?;
        ensure_checkpoint_profiles(&self.manifest, hnsw_profile, vector_profile)?;

        fs::create_dir_all(&self.segments_path)?;
        let new_segment_id = self.manifest.generation + 1;
        let new_segment_path = segment_path(&self.segments_path, new_segment_id);
        SegmentWriter::write_refs(&new_segment_path, &cells)?;

        let index = EngineAqlIndex::try_from_segment_cell_refs(&cells)?;
        index
            .bitmap_index()
            .write(bitmap_path(&self.segments_path, new_segment_id))?;
        index
            .lexical_index()
            .write(lexical_path(&self.segments_path, new_segment_id))?;
        super::vector::vector_index_for_cell_refs(&cells)
            .write(vector_path(&self.segments_path, new_segment_id))?;
        if self.feature_flags.experimental_hnsw {
            super::hnsw::hnsw_graph_for_cell_refs_with_config(&cells, self.hnsw_build_config)?
                .write(hnsw_path(&self.segments_path, new_segment_id))?;
        }

        let output_bytes = self.segment_bundle_bytes(new_segment_id);
        let selected_segments: Vec<ManifestSegment> = selected_indices
            .iter()
            .map(|i| self.manifest.live_segments[*i].clone())
            .collect();
        let new_segment = ManifestSegment {
            id: new_segment_id,
            generation: self.manifest.generation + 1,
            checkpoint_seq: self.manifest.checkpoint_seq,
            cell_count: segment_cell_count(cells.len())?,
        };
        self.manifest
            .replace_segments(selected_segments, new_segment);

        let duration_ms = start.elapsed().as_millis() as u64;
        self.manifest.compaction_metadata.completed += 1;
        self.manifest.compaction_metadata.duration_ms += duration_ms;
        self.manifest.compaction_metadata.cells_compacted += cells.len() as u64;
        self.manifest.compaction_metadata.input_bytes += input_bytes;
        self.manifest.store(&self.manifest_path)?;
        self.invalidate_persisted_index_cache();
        self.memtable.gc_versions_before(self.gc_horizon());

        Ok(CompactionStats {
            segments_before,
            segments_after: self.manifest.live_segments.len(),
            cells_compacted: cells.len(),
            input_bytes,
            output_bytes,
            duration_ms,
        })
    }

    fn select_segments_for_compaction(&self) -> Vec<usize> {
        let live = &self.manifest.live_segments;
        if live.len() < self.compaction_policy.trigger_segment_count.max(2) {
            return Vec::new();
        }
        let mut selected = Vec::new();
        let mut input_bytes: u64 = 0;
        for (idx, segment) in live.iter().enumerate() {
            if selected.len() >= self.compaction_policy.max_input_segments {
                break;
            }
            let bytes = self.segment_bundle_bytes(segment.id);
            if selected.len() >= 2
                && input_bytes.saturating_add(bytes) > self.compaction_policy.max_input_bytes
            {
                break;
            }
            selected.push(idx);
            input_bytes = input_bytes.saturating_add(bytes);
        }
        if selected.len() < 2 {
            Vec::new()
        } else {
            selected
        }
    }

    fn segment_bundle_bytes(&self, segment_id: u64) -> u64 {
        let bundle = SegmentBundle::new(&self.segments_path, segment_id);
        if !bundle.exists_all() {
            return 0;
        }
        [
            &bundle.segment_path,
            &bundle.bitmap_path,
            &bundle.lexical_path,
            &bundle.vector_path,
            &bundle.hnsw_path,
        ]
        .iter()
        .filter_map(|path| fs::metadata(path).ok().map(|m| m.len()))
        .sum()
    }

    fn invalidate_persisted_index_cache(&self) {
        if let Ok(mut cache) = self.persisted_index_cache.lock() {
            *cache = None;
        }
    }
}

fn build_cell_refs(
    merged: &BTreeMap<CellId, SegmentCellRecord>,
) -> EngineResult<Vec<SegmentCellRef<'_>>> {
    let mut cells = Vec::with_capacity(merged.len());
    for (ordinal, (cell_id, record)) in merged.iter().enumerate() {
        let candidate_id = candidate_from_ordinal(ordinal)?;
        let descriptor = record
            .descriptor
            .as_deref()
            .and_then(CellDescriptor::decode_section_v1)
            .map(|descriptor| descriptor.encode_section_v1());
        cells.push(SegmentCellRef {
            candidate_id,
            cell_id: cell_id.0,
            created_seq: record.cell.created_seq,
            deleted_seq: None,
            descriptor,
            payload: &record.cell.payload,
        });
    }
    cells.sort_by_key(|cell| cell.candidate_id);
    Ok(cells)
}
