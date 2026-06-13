use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use cortex_aql::BitmapProvider;
use cortex_core::memtable::{CellVersion, MemTable, ReadTxn};
use cortex_core::{CellDescriptor, CellId, CommitSeq};
use cortex_storage::manifest::StorageManifest;
use cortex_storage::wal::{DurabilityMode, WalWriterHandle};

use crate::checkpoint::PersistedIndexCache;
use crate::feedback::FeedbackIndex;
use crate::lock::DatabaseLock;
use crate::options::{CompactionPolicy, EngineFeatureFlags};
use crate::query::cache::AqlQueryCache;
use crate::query::AqlDeltaIndex;
use crate::query::CellMetadata;
use crate::search::{CorpusSynonymStore, HnswBuildConfig};
use crate::session::SessionIndex;
use crate::tool_registry::ToolIndex;
use crate::verification::TemporalFactStore;

pub trait CandidateResolver: BitmapProvider {
    fn cell_id_for_candidate(&self, candidate: u32) -> Option<CellId>;
}

#[derive(Debug)]
pub struct Database {
    pub(crate) root_path: PathBuf,
    pub(crate) wal_path: PathBuf,
    pub(crate) manifest_path: PathBuf,
    pub(crate) segments_path: PathBuf,
    pub(crate) manifest: StorageManifest,
    pub(crate) memtable: MemTable,
    pub(crate) writer: WalWriterHandle,
    pub(crate) current_seq: CommitSeq,
    pub(crate) durability_mode: DurabilityMode,
    pub(crate) hnsw_build_config: HnswBuildConfig,
    pub(crate) feature_flags: EngineFeatureFlags,
    pub(crate) ingestion_backpressure_policy: crate::ingestion::IngestionBackpressurePolicy,
    pub(crate) ingestion_rate_state: Mutex<crate::ingestion::IngestionRateState>,
    pub(crate) aql_query_cache: Mutex<AqlQueryCache>,
    pub(crate) aql_delta_index: AqlDeltaIndex,
    pub(crate) corpus_synonym_store: CorpusSynonymStore,
    pub(crate) feedback_index: FeedbackIndex,
    pub(crate) session_index: SessionIndex,
    pub(crate) temporal_fact_store: TemporalFactStore,
    pub(crate) tool_index: ToolIndex,
    pub(crate) persisted_index_cache: Mutex<Option<PersistedIndexCache>>,
    pub(crate) active_read_pins: Arc<Mutex<BTreeMap<CommitSeq, usize>>>,
    pub(crate) compaction_policy: CompactionPolicy,
    pub(crate) _lock: DatabaseLock,
    pub(crate) closed: bool,
}

/// A pinned read transaction that prevents GC from removing versions visible
/// to this snapshot until it is dropped.
#[derive(Debug)]
pub struct PinnedReadTxn {
    pub(crate) read_txn: ReadTxn,
    pub(crate) registry: Arc<Mutex<BTreeMap<CommitSeq, usize>>>,
}

impl PinnedReadTxn {
    pub fn read_txn(&self) -> ReadTxn {
        self.read_txn
    }
}

impl Drop for PinnedReadTxn {
    fn drop(&mut self) {
        if let Ok(mut pins) = self.registry.lock() {
            let seq = self.read_txn.read_seq;
            if let Some(count) = pins.get_mut(&seq) {
                if *count > 1 {
                    *count -= 1;
                } else {
                    pins.remove(&seq);
                }
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RetrievedCell {
    pub cell_id: CellId,
    pub payload: Vec<u8>,
    pub descriptor: CellDescriptor,
}

impl RetrievedCell {
    pub fn from_payload(cell_id: CellId, payload: Vec<u8>) -> Self {
        let descriptor = CellDescriptor::from_payload_lossy(&payload);
        Self {
            cell_id,
            payload,
            descriptor,
        }
    }

    pub(crate) fn from_version(version: &CellVersion) -> Self {
        Self {
            cell_id: version.cell_id,
            payload: version.payload.clone(),
            descriptor: version.descriptor.clone(),
        }
    }

    pub(crate) fn metadata(&self) -> CellMetadata {
        CellMetadata::from_payload_with_descriptor(&self.payload, &self.descriptor)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CheckpointStats {
    pub segment_id: Option<u64>,
    pub cells_flushed: usize,
    pub checkpoint_seq: CommitSeq,
}

impl Drop for Database {
    fn drop(&mut self) {
        if !self.closed {
            let _ = self.writer.shutdown();
        }
    }
}
