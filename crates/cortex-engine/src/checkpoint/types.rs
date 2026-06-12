use std::collections::BTreeMap;
use std::sync::Arc;

use cortex_core::memtable::MemTable;
use cortex_core::CellId;
use cortex_storage::indexes::{BitmapIndex, LexicalIndex};
use cortex_storage::manifest::StorageManifest;

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
    pub(crate) key: Vec<(u64, u64, u64)>,
    pub(crate) state: Arc<PersistedIndexState>,
}
