use std::collections::BTreeSet;
use std::path::Path;
use std::sync::Arc;

use cortex_aql::ScopeId;
use cortex_core::CellId;
use cortex_storage::indexes::{BitmapIndex, LexicalIndex};
use cortex_storage::segment::SegmentReader;

use crate::database::Database;
use crate::error::EngineResult;
use crate::query::AqlPermissionPruning;

use super::index_state::{merge_bitmap_index, merge_lexical_index, remove_candidates};
use super::paths::{bitmap_path, lexical_path, segment_path};
use super::types::{PersistedIndexCache, PersistedIndexCacheStats, PersistedIndexState};

const LARGE_LEXICAL_TERMS_ONLY_THRESHOLD_BYTES: u64 = 512 * 1024 * 1024;

impl Database {
    pub fn persisted_indexes(&self) -> EngineResult<(BitmapIndex, LexicalIndex)> {
        let state = self.persisted_index_state()?;
        Ok((state.bitmap, state.lexical))
    }

    pub(crate) fn persisted_index_state(&self) -> EngineResult<PersistedIndexState> {
        let mut state = PersistedIndexState::default();
        let mut tombstoned = BTreeSet::new();
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
            remove_candidates(&mut state, &segment_candidates);
            for entry in entries {
                if entry.deleted {
                    tombstoned.insert(entry.candidate_id);
                    state.candidate_to_cell.remove(&entry.candidate_id);
                } else {
                    tombstoned.remove(&entry.candidate_id);
                    state
                        .candidate_to_cell
                        .insert(entry.candidate_id, CellId(entry.cell_id));
                }
            }
            let segment_bitmap = BitmapIndex::read(bitmap_path(&self.segments_path, segment.id))?;
            let segment_lexical =
                read_persisted_lexical_index(&lexical_path(&self.segments_path, segment.id))?;
            merge_bitmap_index(&mut state, segment_bitmap);
            merge_lexical_index(&mut state, segment_lexical);
        }
        remove_candidates(&mut state, &tombstoned);
        Ok(state)
    }

    pub(crate) fn persisted_index_state_for_readable_scopes(
        &self,
        readable_scopes: &BTreeSet<ScopeId>,
    ) -> EngineResult<(PersistedIndexState, AqlPermissionPruning)> {
        let total_segments = self.manifest.live_segments.len();
        let Some(open_segment_ids) = self
            .statistics()
            .segments_matching_any_scope_id(readable_scopes)
        else {
            let state = (*self.persisted_index_state_cached()?).clone();
            return Ok((
                state,
                AqlPermissionPruning {
                    total_segments,
                    opened_segments: total_segments,
                    skipped_segments: 0,
                },
            ));
        };
        let open_segment_ids = open_segment_ids.into_iter().collect::<BTreeSet<_>>();
        if open_segment_ids.len() == total_segments {
            let state = (*self.persisted_index_state_cached()?).clone();
            return Ok((
                state,
                AqlPermissionPruning {
                    total_segments,
                    opened_segments: total_segments,
                    skipped_segments: 0,
                },
            ));
        }

        let state = self.persisted_index_state_for_segments(&open_segment_ids)?;
        Ok((
            state,
            AqlPermissionPruning {
                total_segments,
                opened_segments: open_segment_ids.len(),
                skipped_segments: total_segments.saturating_sub(open_segment_ids.len()),
            },
        ))
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
        let key = self.persisted_index_key();
        let mut cache = self
            .persisted_index_cache
            .lock()
            .expect("persisted index cache mutex poisoned");
        if let Some(entry) = cache.as_ref() {
            if entry.key == key {
                return Ok(Arc::clone(&entry.state));
            }
        }
        if let Some(entry) = cache.as_mut() {
            if key_starts_with(&key, &entry.key) {
                let old_len = entry.key.len();
                let suffix = self.manifest.live_segments[old_len..].to_vec();
                let state = Arc::make_mut(&mut entry.state);
                for segment in suffix {
                    self.apply_persisted_segment_delta(state, segment.id)?;
                    entry.stats.incremental_segments += 1;
                }
                entry.key = key;
                return Ok(Arc::clone(&entry.state));
            }
        }
        let previous_stats = cache.as_ref().map(|entry| entry.stats).unwrap_or_default();
        let state = Arc::new(self.persisted_index_state()?);
        *cache = Some(PersistedIndexCache {
            key,
            state: Arc::clone(&state),
            stats: PersistedIndexCacheStats {
                full_rebuilds: previous_stats.full_rebuilds + 1,
                incremental_segments: previous_stats.incremental_segments,
            },
        });
        Ok(state)
    }

    fn persisted_index_key(&self) -> Vec<(u64, u64, u64)> {
        self.manifest
            .live_segments
            .iter()
            .map(|segment| (segment.id, segment.generation, segment.checkpoint_seq))
            .collect()
    }

    fn apply_persisted_segment_delta(
        &self,
        state: &mut PersistedIndexState,
        segment_id: u64,
    ) -> EngineResult<()> {
        let entries =
            SegmentReader::read_candidate_entries(segment_path(&self.segments_path, segment_id))?;
        let segment_candidates = entries
            .iter()
            .map(|entry| entry.candidate_id)
            .collect::<BTreeSet<_>>();
        remove_candidates(state, &segment_candidates);
        let mut tombstoned = BTreeSet::new();
        for entry in entries {
            if entry.deleted {
                tombstoned.insert(entry.candidate_id);
                state.candidate_to_cell.remove(&entry.candidate_id);
            } else {
                state
                    .candidate_to_cell
                    .insert(entry.candidate_id, CellId(entry.cell_id));
            }
        }
        let segment_bitmap = BitmapIndex::read(bitmap_path(&self.segments_path, segment_id))?;
        let segment_lexical =
            read_persisted_lexical_index(&lexical_path(&self.segments_path, segment_id))?;
        merge_bitmap_index(state, segment_bitmap);
        merge_lexical_index(state, segment_lexical);
        remove_candidates(state, &tombstoned);
        Ok(())
    }

    fn persisted_index_state_for_segments(
        &self,
        open_segment_ids: &BTreeSet<u64>,
    ) -> EngineResult<PersistedIndexState> {
        let mut state = PersistedIndexState::default();
        for segment in &self.manifest.live_segments {
            let entries = SegmentReader::read_candidate_entries(segment_path(
                &self.segments_path,
                segment.id,
            ))?;
            let segment_candidates = entries
                .iter()
                .map(|entry| entry.candidate_id)
                .collect::<BTreeSet<_>>();
            remove_candidates(&mut state, &segment_candidates);

            if !open_segment_ids.contains(&segment.id) {
                continue;
            }

            for entry in entries {
                if !entry.deleted {
                    state
                        .candidate_to_cell
                        .insert(entry.candidate_id, CellId(entry.cell_id));
                }
            }
            let segment_bitmap = BitmapIndex::read(bitmap_path(&self.segments_path, segment.id))?;
            let segment_lexical =
                read_persisted_lexical_index(&lexical_path(&self.segments_path, segment.id))?;
            merge_bitmap_index(&mut state, segment_bitmap);
            merge_lexical_index(&mut state, segment_lexical);
        }
        Ok(state)
    }

    #[cfg(test)]
    pub(crate) fn persisted_index_cache_stats(&self) -> Option<PersistedIndexCacheStats> {
        self.persisted_index_cache
            .lock()
            .expect("persisted index cache mutex poisoned")
            .as_ref()
            .map(|entry| entry.stats)
    }
}

fn key_starts_with(key: &[(u64, u64, u64)], prefix: &[(u64, u64, u64)]) -> bool {
    key.len() >= prefix.len() && &key[..prefix.len()] == prefix
}

fn read_persisted_lexical_index(path: &Path) -> EngineResult<LexicalIndex> {
    let size = std::fs::metadata(path)
        .map(|metadata| metadata.len())
        .unwrap_or(0);
    if size >= LARGE_LEXICAL_TERMS_ONLY_THRESHOLD_BYTES {
        Ok(LexicalIndex::read_terms_only(path)?)
    } else {
        Ok(LexicalIndex::read(path)?)
    }
}
