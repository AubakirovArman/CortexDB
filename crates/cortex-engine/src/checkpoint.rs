use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;
use std::sync::Arc;

mod candidates;
pub(crate) mod hnsw;
mod index_merge;
mod manifest_safety;
mod paths;
pub(crate) mod vector;

use cortex_core::memtable::MemTable;
use cortex_core::{CellId, CommitSeq};
use cortex_storage::indexes::{BitmapIndex, LexicalIndex};
use cortex_storage::manifest::{
    ManifestHnswProfile, ManifestSegment, ManifestVectorProfile, StorageManifest,
};
use cortex_storage::segment::{SegmentCell, SegmentReader, SegmentWriter};
use cortex_storage::wal::WalWriter;

use crate::database::{CheckpointStats, Database};
use crate::error::{EngineError, EngineResult};
use crate::query::EngineAqlIndex;
use crate::search::HnswBuildConfig;
use candidates::{candidate_from_ordinal, segment_cell_count, CandidateAllocator};
use index_merge::{merge_bitmap_index, merge_lexical_index};
pub(crate) use paths::{
    bitmap_path, hnsw_path, lexical_path, manifest_path, segment_path, segments_path, vector_path,
};

pub(crate) struct CheckpointLoad {
    pub manifest: StorageManifest,
    pub memtable: MemTable,
}

#[derive(Debug)]
pub(crate) struct PersistedIndexState {
    pub bitmap: BitmapIndex,
    pub lexical: LexicalIndex,
    pub candidate_to_cell: BTreeMap<u32, CellId>,
}

/// Cached `PersistedIndexState` plus the live-segment fingerprint it was built
/// from. The fingerprint `(id, generation, checkpoint_seq)` changes whenever a
/// checkpoint or compaction rewrites the segment set, so a stale cache entry can
/// never be reused after the persisted data changes.
#[derive(Debug)]
pub(crate) struct PersistedIndexCache {
    key: Vec<(u64, u64, u64)>,
    state: Arc<PersistedIndexState>,
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
        let hnsw_profile = self
            .feature_flags
            .experimental_hnsw
            .then(|| manifest_hnsw_profile(self.hnsw_build_config))
            .transpose()?;
        let vector_profile = vector::vector_profile_for_cells(&cells, self.hnsw_build_config)?;
        ensure_checkpoint_profiles(&self.manifest, hnsw_profile, vector_profile)?;

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
        if self.feature_flags.experimental_hnsw {
            hnsw::hnsw_graph_for_cells_with_config(&cells, self.hnsw_build_config)?
                .write(hnsw_path(&self.segments_path, segment_id))?;
        }

        self.manifest.checkpoint_segment(ManifestSegment {
            id: segment_id,
            generation: self.manifest.generation + 1,
            checkpoint_seq: self.current_seq.0,
            cell_count: segment_cell_count(cells.len())?,
        });
        self.manifest.hnsw_profile = hnsw_profile;
        if let Some(profile) = vector_profile {
            self.manifest.vector_profile = Some(profile);
        }
        self.manifest.store(&self.manifest_path)?;
        super::database::truncate_wal_tail(&self.wal_path, 0)?;
        self.writer = WalWriter::start(&self.wal_path, self.durability_mode)?;
        self.memtable.gc_versions_before(self.current_seq);
        Ok(CheckpointStats {
            segment_id: Some(segment_id),
            cells_flushed: cells.len(),
            checkpoint_seq: self.current_seq,
        })
    }

    pub fn compact(&mut self) -> EngineResult<CheckpointStats> {
        let cells = self.full_snapshot_cells()?;
        let hnsw_profile = self
            .feature_flags
            .experimental_hnsw
            .then(|| manifest_hnsw_profile(self.hnsw_build_config))
            .transpose()?;
        let vector_profile = vector::vector_profile_for_cells(&cells, self.hnsw_build_config)?;
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
        if self.feature_flags.experimental_hnsw {
            hnsw::hnsw_graph_for_cells_with_config(&cells, self.hnsw_build_config)?
                .write(hnsw_path(&self.segments_path, segment_id))?;
        }

        self.manifest.compact_to_segment(ManifestSegment {
            id: segment_id,
            generation: self.manifest.generation + 1,
            checkpoint_seq: self.current_seq.0,
            cell_count: segment_cell_count(cells.len())?,
        });
        self.manifest.hnsw_profile = hnsw_profile;
        self.manifest.vector_profile = vector_profile;
        self.manifest.store(&self.manifest_path)?;
        super::database::truncate_wal_tail(&self.wal_path, 0)?;
        self.writer = WalWriter::start(&self.wal_path, self.durability_mode)?;
        self.memtable.gc_versions_before(self.current_seq);
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
            // Index rebuild only needs candidate/cell identity and liveness, so
            // read the lightweight entries instead of copying every payload.
            let entries = SegmentReader::read_candidate_entries(segment_path(
                &self.segments_path,
                segment.id,
            ))?;
            let segment_candidates = entries
                .iter()
                .map(|entry| entry.candidate_id)
                .collect::<BTreeSet<_>>();
            remove_candidates(&mut bitmap, &mut lexical, &segment_candidates);
            for entry in entries {
                if entry.deleted {
                    tombstoned.insert(entry.candidate_id);
                    candidate_to_cell.remove(&entry.candidate_id);
                } else {
                    tombstoned.remove(&entry.candidate_id);
                    candidate_to_cell.insert(entry.candidate_id, CellId(entry.cell_id));
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

    /// Return the persisted index, reusing a cached copy when the live-segment
    /// set is unchanged.
    ///
    /// `persisted_index_state` rereads and re-decodes every live segment (the
    /// `.acs`, `.acb`, and `.aci` files), which for a large checkpointed corpus
    /// is multi-gigabyte work. Without caching, a reopened database paid that
    /// cost on every search call. The cache key is the live-segment fingerprint,
    /// so a checkpoint or compaction that rewrites segments transparently forces
    /// a rebuild while a steady-state read corpus is decoded only once.
    pub(crate) fn persisted_index_state_cached(&self) -> EngineResult<Arc<PersistedIndexState>> {
        let key: Vec<(u64, u64, u64)> = self
            .manifest
            .live_segments
            .iter()
            .map(|segment| (segment.id, segment.generation, segment.checkpoint_seq))
            .collect();
        let mut cache = self
            .persisted_index_cache
            .lock()
            .expect("persisted index cache mutex poisoned");
        if let Some(entry) = cache.as_ref() {
            if entry.key == key {
                return Ok(Arc::clone(&entry.state));
            }
        }
        let state = Arc::new(self.persisted_index_state()?);
        *cache = Some(PersistedIndexCache {
            key,
            state: Arc::clone(&state),
        });
        Ok(state)
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

    pub(crate) fn full_snapshot_cells(&self) -> EngineResult<Vec<SegmentCell>> {
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

pub(crate) fn manifest_hnsw_profile(config: HnswBuildConfig) -> EngineResult<ManifestHnswProfile> {
    let config = config.normalized();
    Ok(ManifestHnswProfile {
        max_neighbors: hnsw_profile_u32("max_neighbors", config.max_neighbors)?,
        ef_search: hnsw_profile_u32("ef_search", config.ef_search)?,
        layer_count: hnsw_profile_u32("layer_count", config.layer_count)?,
        metric: config.metric as u32,
        ef_construction: hnsw_profile_u32("ef_construction", config.ef_construction)?,
    })
}

fn ensure_checkpoint_profiles(
    manifest: &StorageManifest,
    hnsw_profile: Option<ManifestHnswProfile>,
    vector_profile: Option<ManifestVectorProfile>,
) -> EngineResult<()> {
    if manifest.live_segments.is_empty() {
        return Ok(());
    }
    if let (Some(existing), Some(next)) = (manifest.hnsw_profile, hnsw_profile) {
        if existing != next {
            return Err(EngineError::StorageInvariant(format!(
                "checkpoint hnsw profile {:?} does not match existing manifest profile {:?}; run compact with the new HNSW build config first",
                next, existing
            )));
        }
    }
    if let (Some(existing), Some(next)) = (manifest.vector_profile, vector_profile) {
        if existing != next {
            return Err(EngineError::StorageInvariant(format!(
                "checkpoint vector profile {:?} does not match existing manifest profile {:?}; run compact with a consistent vector collection profile first",
                next, existing
            )));
        }
    }
    Ok(())
}

fn hnsw_profile_u32(field: &'static str, value: usize) -> EngineResult<u32> {
    u32::try_from(value).map_err(|_| EngineError::HnswBuildConfigOutOfRange { field, value })
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
    manifest_safety::reject_missing_manifest_with_storage(root)?;
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

#[cfg(test)]
mod persisted_index_cache_tests {
    use std::sync::Arc;

    use cortex_core::{CellId, KnowledgeCell, KnowledgeCellMetadata, KnowledgeCellType};

    use crate::Database;

    fn cell(body: &str) -> KnowledgeCell {
        KnowledgeCell::new(
            KnowledgeCellMetadata {
                scope: "project:cache".to_owned(),
                status: "ready".to_owned(),
                cell_type: KnowledgeCellType::Fact,
                memory_type: None,
                ttl_seconds: None,
                created_unix_seconds: None,
                source_trust_q16: None,
                source: Some("cache-test".to_owned()),
            },
            body,
        )
    }

    #[test]
    fn reopened_checkpointed_db_reuses_persisted_index_state_across_calls() {
        let dir = tempfile::tempdir().unwrap();
        {
            let mut db = Database::open(dir.path()).unwrap();
            db.put_knowledge_cell(CellId(1), cell("solar plant budget approved"))
                .unwrap();
            db.put_knowledge_cell(CellId(2), cell("wind farm budget approved"))
                .unwrap();
            db.checkpoint().unwrap();
        }

        // Reopen: WAL is truncated and the memtable is empty, so searches take
        // the persisted fast path. The decoded index must be cached, not rebuilt
        // from the multi-gigabyte segment files on every call.
        let db = Database::open(dir.path()).unwrap();
        let first = db.persisted_index_state_cached().unwrap();
        let second = db.persisted_index_state_cached().unwrap();
        assert!(
            Arc::ptr_eq(&first, &second),
            "second call must reuse the cached persisted index, not rebuild it"
        );
        assert!(!first.candidate_to_cell.is_empty());
    }

    #[test]
    fn checkpoint_that_changes_segments_invalidates_cached_persisted_index() {
        let dir = tempfile::tempdir().unwrap();
        let mut db = Database::open(dir.path()).unwrap();
        db.put_knowledge_cell(CellId(1), cell("solar plant budget approved"))
            .unwrap();
        db.checkpoint().unwrap();
        let before = db.persisted_index_state_cached().unwrap();

        // A new checkpoint that writes another segment changes the live-segment
        // fingerprint, so the cache key no longer matches and the state is rebuilt.
        db.put_knowledge_cell(CellId(2), cell("wind farm budget approved"))
            .unwrap();
        db.checkpoint().unwrap();
        let after = db.persisted_index_state_cached().unwrap();

        assert!(
            !Arc::ptr_eq(&before, &after),
            "a checkpoint that rewrites segments must invalidate the cached index"
        );
        assert_eq!(after.candidate_to_cell.len(), 2);
    }
}
