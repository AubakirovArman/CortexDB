use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use cortex_core::memtable::MemTable;
use cortex_core::CellId;
use cortex_storage::indexes::{BitmapIndex, LexicalIndex};
use cortex_storage::manifest::StorageManifest;

pub(crate) struct CheckpointLoad {
    pub manifest: StorageManifest,
    pub memtable: MemTable,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct PersistedIndexState {
    pub bitmap: BitmapIndex,
    pub lexical: LexicalIndex,
    pub candidate_to_cell: BTreeMap<u32, CellId>,
    pub postings: PersistedIndexPostings,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct PersistedIndexPostings {
    pub bitmap_handles_by_candidate: BTreeMap<u32, BTreeSet<u64>>,
    pub lexical_terms_by_candidate: BTreeMap<u32, BTreeSet<String>>,
    pub lexical_fields_by_candidate: BTreeMap<u32, BTreeSet<String>>,
    pub lexical_field_terms_by_candidate: BTreeMap<u32, BTreeSet<(String, String)>>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct PersistedIndexCacheStats {
    pub full_rebuilds: u64,
    pub incremental_segments: u64,
}

/// Cached `PersistedIndexState` plus the live-segment fingerprint it was built
/// from. The fingerprint `(id, generation, checkpoint_seq)` changes whenever a
/// checkpoint or compaction rewrites the segment set, so a stale cache entry can
/// never be reused after the persisted data changes.
#[derive(Debug)]
pub(crate) struct PersistedIndexCache {
    pub(crate) key: Vec<(u64, u64, u64)>,
    pub(crate) state: Arc<PersistedIndexState>,
    pub(crate) stats: PersistedIndexCacheStats,
}
