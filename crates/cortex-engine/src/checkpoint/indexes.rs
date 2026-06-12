use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::Arc;

use cortex_core::CellId;
use cortex_storage::indexes::{BitmapIndex, LexicalIndex};
use cortex_storage::segment::SegmentReader;

use crate::database::Database;
use crate::error::EngineResult;

use super::index_merge::{merge_bitmap_index, merge_lexical_index};
use super::paths::{bitmap_path, lexical_path, segment_path};
use super::types::{PersistedIndexCache, PersistedIndexState};

const LARGE_LEXICAL_TERMS_ONLY_THRESHOLD_BYTES: u64 = 512 * 1024 * 1024;

impl Database {
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
                read_persisted_lexical_index(&lexical_path(&self.segments_path, segment.id))?;
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
    for values in lexical.field_doc_lengths.values_mut() {
        values.retain(|candidate, _| !candidates.contains(candidate));
    }
    lexical
        .field_doc_lengths
        .retain(|_, values| !values.is_empty());
    for terms in lexical.field_term_frequencies.values_mut() {
        for values in terms.values_mut() {
            values.retain(|candidate, _| !candidates.contains(candidate));
        }
        terms.retain(|_, values| !values.is_empty());
    }
    lexical
        .field_term_frequencies
        .retain(|_, terms| !terms.is_empty());
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
