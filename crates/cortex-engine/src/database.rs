use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use crate::exec::{execute_retrieve, RetrieveExecutionReport};
use crate::lock::DatabaseLock;

#[cfg(test)]
use cortex_aql::eval_bitmap_program;
use cortex_aql::{BitmapProvider, BoundRetrievePlan, QualityThresholds};
use cortex_core::memtable::{CellVersion, MemTable, ReadTxn};
use cortex_core::{CellDescriptor, CellId, CommitSeq};
use cortex_storage::manifest::StorageManifest;
use cortex_storage::wal::{DurabilityMode, WalWriter, WalWriterHandle};

use crate::checkpoint::{load_checkpoint, manifest_path, segments_path, PersistedIndexCache};
use crate::cleanup::{cleanup_orphans, remove_lock_file};
use crate::database_files::find_wal_files;
pub(crate) use crate::database_files::truncate_wal_tail;
use crate::error::{EngineError, EngineResult};
use crate::operation::{
    wal_record_from_operation_with_metadata, wal_record_from_operation_with_seq, DbOperation,
};
use crate::options::{
    DatabaseOptions, EngineFeature, EngineFeatureFlags, RecoveryMode, StaleLockPolicy,
};
use crate::query::cache::AqlQueryCache;
use crate::query::CellMetadata;
use crate::replay::{replay_wal_best_effort_into, replay_wal_into};
use crate::search::{analyze_search_query, HnswBuildConfig};
use crate::source_trust::SourceTrust;

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
    pub(crate) persisted_index_cache: Mutex<Option<PersistedIndexCache>>,
    pub(crate) active_read_pins: Arc<Mutex<BTreeMap<CommitSeq, usize>>>,
    pub(crate) _lock: DatabaseLock,
    closed: bool,
}

/// A pinned read transaction that prevents GC from removing versions visible
/// to this snapshot until it is dropped.
#[derive(Debug)]
pub struct PinnedReadTxn {
    read_txn: ReadTxn,
    registry: Arc<Mutex<BTreeMap<CommitSeq, usize>>>,
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

impl Database {
    /// Open a database at the given path with default options.
    ///
    /// # Example
    ///
    /// ```
    /// # use std::path::Path;
    /// # let dir = tempfile::tempdir().unwrap();
    /// # let path = dir.path();
    /// use cortex_engine::Database;
    /// let db = Database::open(path).unwrap();
    /// ```
    pub fn open(path: impl AsRef<Path>) -> EngineResult<Self> {
        Self::open_with_options(path, DatabaseOptions::default())
    }

    pub fn break_stale_lock(path: impl AsRef<Path>) -> EngineResult<()> {
        remove_lock_file(path.as_ref())
    }

    pub fn feature_flags(&self) -> EngineFeatureFlags {
        self.feature_flags
    }

    pub(crate) fn require_feature(&self, feature: EngineFeature) -> EngineResult<()> {
        if self.feature_flags.is_enabled(feature) {
            Ok(())
        } else {
            Err(EngineError::FeatureDisabled(feature.as_str()))
        }
    }

    pub fn open_with_options(
        path: impl AsRef<Path>,
        options: DatabaseOptions,
    ) -> EngineResult<Self> {
        let root_path = path.as_ref().to_owned();
        std::fs::create_dir_all(&root_path)?;
        let lock = match DatabaseLock::acquire(&root_path) {
            Ok(lock) => lock,
            Err(EngineError::DatabaseAlreadyOpen(_))
                if options.stale_lock_policy == StaleLockPolicy::Break =>
            {
                Self::break_stale_lock(&root_path)?;
                DatabaseLock::acquire(&root_path)?
            }
            Err(error) => return Err(error),
        };
        cleanup_orphans(&root_path)?;
        let wal_path = root_path.join("db.aclog");
        let manifest_path = manifest_path(&root_path);
        let segments_path = segments_path(&root_path);
        let checkpoint = load_checkpoint(&root_path)?;
        let wal_files = find_wal_files(&wal_path);
        let mut current_memtable = checkpoint.memtable;
        let mut current_seq = CommitSeq(checkpoint.manifest.checkpoint_seq);
        let mut last_safe_offset = 0;

        for file in &wal_files {
            let replay = match options.recovery_mode {
                RecoveryMode::Strict => replay_wal_into(file, current_memtable, current_seq)?,
                RecoveryMode::BestEffort => {
                    replay_wal_best_effort_into(file, current_memtable, current_seq)?
                }
            };
            current_memtable = replay.memtable;
            current_seq = replay.last_seq;
            if file == &wal_path {
                last_safe_offset = replay.safe_truncate_offset;
            }
        }

        truncate_wal_tail(&wal_path, last_safe_offset)?;
        let writer = WalWriter::start(&wal_path, options.durability_mode)?;
        let database = Self {
            root_path,
            wal_path,
            manifest_path,
            segments_path,
            manifest: checkpoint.manifest,
            memtable: current_memtable,
            writer,
            current_seq,
            durability_mode: options.durability_mode,
            hnsw_build_config: options.hnsw_build_config.normalized(),
            feature_flags: options.feature_flags,
            ingestion_backpressure_policy: options.ingestion_backpressure,
            ingestion_rate_state: crate::ingestion::default_ingestion_rate_state(),
            aql_query_cache: Mutex::new(AqlQueryCache::default()),
            persisted_index_cache: Mutex::new(None),
            active_read_pins: Arc::new(Mutex::new(BTreeMap::new())),
            _lock: lock,
            closed: false,
        };
        database.resume_interrupted_ingestion_jobs()?;
        Ok(database)
    }

    /// Store a single cell payload and return the commit sequence.
    ///
    /// # Example
    ///
    /// ```
    /// # use cortex_engine::Database;
    /// # use cortex_core::CellId;
    /// # let dir = tempfile::tempdir().unwrap();
    /// # let mut db = Database::open(dir.path()).unwrap();
    /// let seq = db.put_cell(CellId(1), b"hello world".to_vec()).unwrap();
    /// assert!(seq.0 > 0);
    /// ```
    pub fn put_cell(&mut self, cell_id: CellId, payload: Vec<u8>) -> EngineResult<CommitSeq> {
        self.append_then_apply(DbOperation::PutCell { cell_id, payload })
    }

    pub fn put_cells(&mut self, cells: Vec<(CellId, Vec<u8>)>) -> EngineResult<CommitSeq> {
        if cells.is_empty() {
            return Ok(self.current_seq);
        }
        let mut records = Vec::with_capacity(cells.len());
        let mut ops = Vec::with_capacity(cells.len());
        let mut next_seq = self.current_seq.0;

        for (cell_id, payload) in cells {
            next_seq += 1;
            let op = DbOperation::PutCell { cell_id, payload };
            let record = wal_record_from_operation_with_seq(CommitSeq(next_seq), &op);
            records.push(record);
            ops.push((CommitSeq(next_seq), op));
        }

        self.writer.append_batch(records)?;

        for (seq, op) in ops {
            self.apply_operation(seq, op)?;
        }
        self.current_seq = CommitSeq(next_seq);
        Ok(self.current_seq)
    }

    pub fn patch_cell(&mut self, cell_id: CellId, payload: Vec<u8>) -> EngineResult<CommitSeq> {
        self.require_visible_cell(cell_id)?;
        self.append_then_apply(DbOperation::PatchCell { cell_id, payload })
    }

    pub fn tombstone_cell(&mut self, cell_id: CellId) -> EngineResult<CommitSeq> {
        self.require_visible_cell(cell_id)?;
        self.append_then_apply(DbOperation::TombstoneCell { cell_id })
    }

    pub fn allocate_cell_id(&self) -> CellId {
        let max_id = self.memtable.max_cell_id().map(|id| id.0).unwrap_or(0);
        CellId(max_id.max(1000) + 1)
    }

    pub fn allocate_cell_id_range(&self, _count: usize) -> CellId {
        let max_id = self.memtable.max_cell_id().map(|id| id.0).unwrap_or(0);
        CellId(max_id.max(10000) + 1)
    }

    pub fn read_txn(&self) -> ReadTxn {
        ReadTxn {
            read_seq: self.current_seq,
        }
    }

    /// Pin a read transaction so that GC does not remove versions visible to
    /// this snapshot until the returned handle is dropped.
    pub fn pin_read_txn(&self) -> PinnedReadTxn {
        let read_txn = self.read_txn();
        let mut pins = self
            .active_read_pins
            .lock()
            .expect("read pin lock poisoned");
        *pins.entry(read_txn.read_seq).or_insert(0) += 1;
        PinnedReadTxn {
            read_txn,
            registry: Arc::clone(&self.active_read_pins),
        }
    }

    /// The oldest snapshot that must be preserved. GC may remove versions
    /// older than this sequence.
    pub fn gc_horizon(&self) -> CommitSeq {
        let pins = self
            .active_read_pins
            .lock()
            .expect("read pin lock poisoned");
        pins.keys().next().copied().unwrap_or(self.current_seq)
    }

    pub fn get_cell(&self, txn: ReadTxn, cell_id: CellId) -> Option<Vec<u8>> {
        self.memtable
            .read(txn, cell_id)
            .map(|version| version.payload.clone())
    }

    /// Read the latest visible payload for a cell.
    ///
    /// # Example
    ///
    /// ```
    /// # use cortex_engine::Database;
    /// # use cortex_core::CellId;
    /// # let dir = tempfile::tempdir().unwrap();
    /// # let mut db = Database::open(dir.path()).unwrap();
    /// db.put_cell(CellId(1), b"hello world".to_vec()).unwrap();
    /// let payload = db.get_latest_cell(CellId(1)).unwrap();
    /// assert_eq!(payload, b"hello world");
    /// ```
    pub fn get_latest_cell(&self, cell_id: CellId) -> Option<Vec<u8>> {
        self.get_cell(self.read_txn(), cell_id)
    }

    pub fn get_latest_cell_with_descriptor(
        &self,
        cell_id: CellId,
    ) -> Option<(Vec<u8>, CellDescriptor)> {
        self.memtable
            .read(self.read_txn(), cell_id)
            .map(|version| (version.payload.clone(), version.descriptor.clone()))
    }

    pub fn retrieve_cells<P: CandidateResolver>(
        &self,
        plan: &BoundRetrievePlan,
        provider: &P,
    ) -> EngineResult<Vec<RetrievedCell>> {
        self.retrieve_cells_with_execution_trace(plan, provider)
            .map(|report| report.cells)
    }

    pub(crate) fn retrieve_cells_with_execution_trace<P: CandidateResolver>(
        &self,
        plan: &BoundRetrievePlan,
        provider: &P,
    ) -> EngineResult<RetrieveExecutionReport> {
        execute_retrieve(self, plan, provider)
    }

    #[cfg(test)]
    pub(crate) fn retrieve_cells_direct<P: CandidateResolver>(
        &self,
        plan: &BoundRetrievePlan,
        provider: &P,
    ) -> EngineResult<Vec<RetrievedCell>> {
        let candidates = eval_bitmap_program(&plan.bitmap_program, provider)?;
        let txn = self.read_txn();
        let cells = candidates
            .into_iter()
            .filter_map(|candidate| provider.cell_id_for_candidate(candidate))
            .filter_map(|cell_id| {
                self.memtable
                    .read(txn, cell_id)
                    .filter(|version| {
                        cell_version_meets_quality_thresholds(version, &plan.quality_thresholds)
                    })
                    .map(RetrievedCell::from_version)
            })
            .collect::<Vec<_>>();
        let ranked =
            suppress_duplicate_content(rank_retrieved_cells(cells, &plan.task, &plan.weights));
        Ok(expand_parent_context(ranked)
            .into_iter()
            .take(plan.context_policy.candidate_limit as usize)
            .collect())
    }

    pub fn rerank_retrieved_cells_for_task(
        &self,
        cells: Vec<RetrievedCell>,
        task: &str,
        weights: &cortex_aql::RetrievalWeights,
    ) -> Vec<RetrievedCell> {
        rank_retrieved_cells(cells, task, weights)
    }

    pub fn current_seq(&self) -> CommitSeq {
        self.current_seq
    }

    pub fn root_path(&self) -> &Path {
        &self.root_path
    }

    pub fn enterprise_system_check(&self) -> EngineResult<bool> {
        let report = self.validate_storage_report();
        if !report.manifest_ok || !report.wal_ok {
            return Ok(false);
        }
        Ok(true)
    }

    pub fn wal_path(&self) -> &Path {
        &self.wal_path
    }

    pub fn manifest(&self) -> &StorageManifest {
        &self.manifest
    }

    pub fn hnsw_no_fallback_rollout_policy(
        &self,
    ) -> Option<crate::search::HnswNoFallbackRolloutPolicy> {
        self.manifest
            .hnsw_no_fallback_profile
            .map(crate::search::HnswNoFallbackRolloutPolicy::from_manifest)
    }

    pub fn set_hnsw_no_fallback_rollout_policy(
        &mut self,
        policy: crate::search::HnswNoFallbackRolloutPolicy,
    ) -> EngineResult<()> {
        let mut manifest = self.manifest.clone();
        manifest.generation += 1;
        manifest.hnsw_no_fallback_profile = Some(policy.to_manifest());
        manifest.store(&self.manifest_path)?;
        self.manifest = manifest;
        Ok(())
    }

    pub fn clear_hnsw_no_fallback_rollout_policy(&mut self) -> EngineResult<()> {
        let mut manifest = self.manifest.clone();
        manifest.generation += 1;
        manifest.hnsw_no_fallback_profile = None;
        manifest.store(&self.manifest_path)?;
        self.manifest = manifest;
        Ok(())
    }

    /// Gracefully shut down the database, flushing WAL and releasing the lock.
    ///
    /// # Example
    ///
    /// ```
    /// # use cortex_engine::Database;
    /// # let dir = tempfile::tempdir().unwrap();
    /// # let db = Database::open(dir.path()).unwrap();
    /// db.close().unwrap();
    /// ```
    pub fn close(mut self) -> EngineResult<()> {
        self.writer.shutdown()?;
        self.closed = true;
        Ok(())
    }

    pub(crate) fn snapshot_versions(&self) -> Vec<CellVersion> {
        self.memtable.visible_cells(self.read_txn())
    }

    fn append_then_apply(&mut self, operation: DbOperation) -> EngineResult<CommitSeq> {
        let next_seq = CommitSeq(self.current_seq.0 + 1);
        let record = wal_record_from_operation_with_seq(next_seq, &operation);
        self.writer.append(record)?;
        self.apply_operation(next_seq, operation)?;
        self.current_seq = next_seq;
        Ok(next_seq)
    }

    pub(crate) fn append_then_apply_with_metadata(
        &mut self,
        operation: DbOperation,
        metadata: Vec<u8>,
    ) -> EngineResult<CommitSeq> {
        let next_seq = CommitSeq(self.current_seq.0 + 1);
        let descriptor =
            (!metadata.is_empty()).then(|| CellDescriptor::from_metadata_section_lossy(&metadata));
        let record = wal_record_from_operation_with_metadata(next_seq, &operation, metadata);
        self.writer.append(record)?;
        self.apply_operation_with_descriptor(next_seq, operation, descriptor)?;
        self.current_seq = next_seq;
        Ok(next_seq)
    }

    fn apply_operation(&mut self, seq: CommitSeq, operation: DbOperation) -> EngineResult<()> {
        self.apply_operation_with_descriptor(seq, operation, None)
    }

    fn apply_operation_with_descriptor(
        &mut self,
        seq: CommitSeq,
        operation: DbOperation,
        descriptor: Option<CellDescriptor>,
    ) -> EngineResult<()> {
        match operation {
            DbOperation::PutCell { cell_id, payload } => {
                if let Some(descriptor) = descriptor {
                    self.memtable
                        .put_cell_with_descriptor(cell_id, seq, payload, descriptor);
                } else {
                    self.memtable.put_cell(cell_id, seq, payload);
                }
                Ok(())
            }
            DbOperation::PatchCell { cell_id, payload } => {
                if let Some(descriptor) = descriptor {
                    self.memtable
                        .patch_cell_with_descriptor(cell_id, seq, payload, descriptor)
                        .map_err(EngineError::from)
                } else {
                    self.memtable
                        .patch_cell(cell_id, seq, payload)
                        .map_err(EngineError::from)
                }
            }
            DbOperation::TombstoneCell { cell_id } => {
                self.memtable.record_tombstone(cell_id, seq);
                Ok(())
            }
        }
    }

    fn require_visible_cell(&self, cell_id: CellId) -> EngineResult<()> {
        self.memtable
            .read(self.read_txn(), cell_id)
            .map(|_| ())
            .ok_or_else(|| cortex_core::CoreError::CellNotFound(cell_id).into())
    }
}

fn metadata_meets_quality_thresholds(
    metadata: &CellMetadata,
    thresholds: &QualityThresholds,
) -> bool {
    let confidence_q16 = metadata
        .source_ref
        .as_ref()
        .map(|source_ref| source_ref.confidence_q16)
        .or(metadata.source_trust_q16)
        .unwrap_or(0);
    if confidence_q16 < thresholds.min_confidence_q16 {
        return false;
    }

    if metadata.source_trust_q16.unwrap_or(0) < thresholds.min_source_trust_q16 {
        return false;
    }

    if let Some(max_freshness_seconds) = thresholds.max_freshness_seconds {
        let Some(created) = metadata.created_unix_seconds else {
            return false;
        };
        let age = unix_now_seconds().saturating_sub(created);
        if age > max_freshness_seconds {
            return false;
        }
    }

    true
}

pub(crate) fn cell_version_meets_quality_thresholds(
    version: &CellVersion,
    thresholds: &QualityThresholds,
) -> bool {
    metadata_meets_quality_thresholds(&CellMetadata::from_version(version), thresholds)
}

fn unix_now_seconds() -> u64 {
    match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
        Ok(duration) => duration.as_secs(),
        Err(_) => 0,
    }
}

pub(crate) fn rank_retrieved_cells(
    mut cells: Vec<RetrievedCell>,
    task: &str,
    weights: &cortex_aql::RetrievalWeights,
) -> Vec<RetrievedCell> {
    if cells.len() <= 1 {
        return cells;
    }

    let metadata = cells
        .iter()
        .map(RetrievedCell::metadata)
        .collect::<Vec<_>>();
    let lexical_scores = lexical_bm25_scores_from_metadata(&metadata, task);
    let query_vector = query_vector_from_task(task);
    let recency_scores = recency_scores_q16_from_metadata(&metadata);
    let rank_keys = cells
        .iter()
        .enumerate()
        .map(|(index, cell)| {
            let lexical = lexical_scores.get(index).copied().unwrap_or(0);
            let semantic = query_vector
                .as_deref()
                .map(|query| semantic_dot_score(&cell.payload, query))
                .unwrap_or(0);
            let recency = recency_scores.get(index).copied().unwrap_or(0);
            let metadata = &metadata[index];
            let trust = u64::from(
                SourceTrust::from_metadata(metadata.source_trust_q16, metadata.source_trust_class)
                    .q16,
            );
            let score = weighted_retrieval_score(lexical, semantic, recency, trust, weights);
            (score, index)
        })
        .collect::<Vec<_>>();
    let mut indexed = cells
        .drain(..)
        .enumerate()
        .map(|(index, cell)| (rank_keys[index], cell))
        .collect::<Vec<_>>();

    indexed.sort_by_key(|((score, index), _)| (Reverse(*score), *index));

    indexed.into_iter().map(|(_, cell)| cell).collect()
}

pub(crate) fn suppress_duplicate_content(cells: Vec<RetrievedCell>) -> Vec<RetrievedCell> {
    let mut seen = BTreeSet::<String>::new();
    cells
        .into_iter()
        .filter(|cell| {
            let metadata = cell.metadata();
            let Some(content_hash) = metadata.content_hash else {
                return true;
            };
            seen.insert(content_hash)
        })
        .collect()
}

pub(crate) fn expand_parent_context(cells: Vec<RetrievedCell>) -> Vec<RetrievedCell> {
    if cells.len() <= 1 {
        return cells;
    }

    let mut parent_key_to_index = BTreeMap::<String, usize>::new();
    let metadata = cells
        .iter()
        .map(RetrievedCell::metadata)
        .collect::<Vec<_>>();
    for (index, metadata) in metadata.iter().enumerate() {
        if let Some(chunk_id) = &metadata.chunk_id {
            parent_key_to_index.entry(chunk_id.clone()).or_insert(index);
        }
        if is_parent_context_metadata(metadata) {
            if let Some(document_id) = &metadata.document_id {
                parent_key_to_index
                    .entry(document_id.clone())
                    .or_insert(index);
            }
        }
    }

    let mut expanded = Vec::with_capacity(cells.len());
    let mut emitted = BTreeSet::<CellId>::new();
    for (index, cell) in cells.iter().enumerate() {
        if emitted.insert(cell.cell_id) {
            expanded.push(cell.clone());
        }
        let Some(parent_id) = metadata[index].parent_id.as_ref() else {
            continue;
        };
        let Some(parent_index) = parent_key_to_index.get(parent_id).copied() else {
            continue;
        };
        let parent = &cells[parent_index];
        if parent.cell_id != cell.cell_id && emitted.insert(parent.cell_id) {
            expanded.push(parent.clone());
        }
    }
    expanded
}

fn is_parent_context_metadata(metadata: &CellMetadata) -> bool {
    metadata
        .chunk_role
        .as_deref()
        .map(|role| {
            role.eq_ignore_ascii_case("parent")
                || role.eq_ignore_ascii_case("document")
                || role.eq_ignore_ascii_case("summary")
        })
        .unwrap_or(false)
}

fn lexical_bm25_scores_from_metadata(metadata: &[CellMetadata], query: &str) -> Vec<u64> {
    let query_terms = analyze_search_query(query).weighted_terms;
    if query_terms.is_empty() || metadata.is_empty() {
        return vec![0; metadata.len()];
    }

    let docs = metadata
        .iter()
        .map(CellMetadata::weighted_lexical_terms)
        .collect::<Vec<_>>();
    let doc_lengths = docs
        .iter()
        .map(|terms| terms.values().copied().sum::<u32>().max(1))
        .collect::<Vec<_>>();
    let avg_len_q10 = average_len_q10(&doc_lengths);
    let doc_count = docs.len() as u64;
    let mut doc_frequency = BTreeMap::<String, u64>::new();
    for term in query_terms.keys() {
        let count = docs
            .iter()
            .filter(|doc| doc.contains_key(term))
            .count()
            .try_into()
            .unwrap_or(u64::MAX);
        doc_frequency.insert(term.clone(), count);
    }

    docs.iter()
        .enumerate()
        .map(|(index, doc)| {
            let len_q10 = u64::from(doc_lengths[index]) * 1024;
            let norm_q10 = 256 + (768 * len_q10 / avg_len_q10.max(1));
            query_terms
                .iter()
                .map(|(term, query_weight)| {
                    let tf = u64::from(*doc.get(term).unwrap_or(&0));
                    if tf == 0 {
                        return 0;
                    }
                    let df = *doc_frequency.get(term).unwrap_or(&0);
                    let idf_q10 = ((doc_count + 1) * 1024) / (df + 1);
                    let denom_q10 = (tf * 1024) + norm_q10;
                    let tf_norm_q10 = (tf * 2048 * 1024) / denom_q10.max(1);
                    idf_q10
                        .saturating_mul(tf_norm_q10)
                        .saturating_mul(u64::from(*query_weight))
                })
                .sum()
        })
        .collect()
}

fn average_len_q10(doc_lengths: &[u32]) -> u64 {
    if doc_lengths.is_empty() {
        return 1024;
    }
    let total = doc_lengths.iter().copied().map(u64::from).sum::<u64>();
    total * 1024 / doc_lengths.len() as u64
}

fn recency_scores_q16_from_metadata(metadata: &[CellMetadata]) -> Vec<u64> {
    let created = metadata
        .iter()
        .map(|metadata| metadata.created_unix_seconds)
        .collect::<Vec<_>>();
    let mut timestamps = created.iter().filter_map(|value| *value);
    let Some(first) = timestamps.next() else {
        return vec![0; metadata.len()];
    };
    let (min, max) = timestamps.fold((first, first), |(min, max), value| {
        (min.min(value), max.max(value))
    });
    if max <= min {
        return created
            .into_iter()
            .map(|value| value.map(|_| u64::from(u16::MAX)).unwrap_or(0))
            .collect();
    }
    let span = max - min;
    created
        .into_iter()
        .map(|value| {
            value
                .map(|created| (created.saturating_sub(min).min(span) * u64::from(u16::MAX)) / span)
                .unwrap_or(0)
        })
        .collect()
}

fn query_vector_from_task(task: &str) -> Option<Vec<i16>> {
    task.lines().find_map(|line| {
        let value = line
            .trim()
            .strip_prefix("query_vector=")
            .or_else(|| line.trim().strip_prefix("vector="))?;
        crate::search::parse_vector_literal(value).ok()
    })
}

fn semantic_dot_score(payload: &[u8], query: &[i16]) -> u64 {
    crate::search::vector::vectors_from_payload(payload)
        .into_iter()
        .filter(|view| view.vector.len() == query.len())
        .map(|view| {
            view.vector
                .iter()
                .zip(query)
                .map(|(left, right)| i64::from(*left) * i64::from(*right))
                .sum::<i64>()
                .max(0) as u64
        })
        .max()
        .unwrap_or(0)
}

fn weighted_retrieval_score(
    lexical: u64,
    semantic: u64,
    recency_q16: u64,
    trust_q16: u64,
    weights: &cortex_aql::RetrievalWeights,
) -> u64 {
    weighted_component(lexical, weights.lexical_q16)
        .saturating_add(weighted_component(semantic, weights.semantic_q16))
        .saturating_add(weighted_component(recency_q16 * 1024, weights.recency_q16))
        .saturating_add(weighted_component(trust_q16 * 1024, weights.trust_q16))
}

fn weighted_component(value: u64, weight_q16: u16) -> u64 {
    value.saturating_mul(u64::from(weight_q16)) / u64::from(u16::MAX)
}

impl Drop for Database {
    fn drop(&mut self) {
        if !self.closed {
            let _ = self.writer.shutdown();
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::time::Instant;

    use cortex_aql::{
        AgentId, AgentView, BoundPlan, BrainId, MemoryType, QualityThresholds, RetrievalMode,
        Q16_ZERO,
    };

    use super::*;
    use crate::query::{scope_id, EngineAqlProvider};

    fn test_view(modes: impl IntoIterator<Item = RetrievalMode>) -> AgentView {
        AgentView {
            agent_id: AgentId(1),
            label: Some("database-test".to_owned()),
            readable_brains: BTreeSet::from([BrainId(1)]),
            readable_scopes: BTreeSet::from([scope_id("default")]),
            writable_scopes: BTreeSet::new(),
            allowed_modes: modes.into_iter().collect(),
            allowed_memory_types: BTreeSet::new(),
            max_context_budget_tokens: 4000,
            default_context_budget_tokens: 1000,
            max_candidate_limit: 100,
            default_candidate_limit: 20,
            min_required_confidence_q16: Q16_ZERO,
            max_ttl_seconds: None,
            allow_remember: false,
            allow_verify_fact: false,
            allow_audit_mode: true,
            require_citations_by_default: false,
            private_scope: None,
        }
    }

    #[test]
    fn retrieve_aql_orders_by_lexical_relevance_before_candidate_id() {
        let dir = tempfile::tempdir().unwrap();
        let mut db = Database::open(dir.path()).unwrap();
        db.put_cell(
            CellId(1),
            b"title=unrelated\n\ncommon body without the important term".to_vec(),
        )
        .unwrap();
        db.put_cell(
            CellId(2),
            b"title=needle migration policy\n\ncommon body".to_vec(),
        )
        .unwrap();

        let view = test_view([RetrievalMode::Balanced]);
        let results = db
            .retrieve_aql(
                r#"RETRIEVE CONTEXT FOR TASK "needle" IN BRAIN default LIMIT 2 CANDIDATES;"#,
                &view,
            )
            .unwrap();

        assert_eq!(results[0].cell_id, CellId(2));
    }

    #[test]
    fn quality_threshold_fast_path_uses_materialized_descriptor() {
        let mut version = CellVersion::new(
            CellId(1),
            CommitSeq(1),
            b"scope=default\nstatus=ready\nsource_trust_q16=60000\n\nbody".to_vec(),
            0,
        );
        version.payload = b"scope=default\nstatus=ready\nsource_trust_q16=1000\n\nbody".to_vec();
        let thresholds = QualityThresholds {
            min_confidence_q16: 0,
            min_source_trust_q16: 50_000,
            max_freshness_seconds: None,
        };

        assert!(cell_version_meets_quality_thresholds(&version, &thresholds));
    }

    #[test]
    fn retrieve_aql_uses_path_view_for_lexical_relevance() {
        let dir = tempfile::tempdir().unwrap();
        let mut db = Database::open(dir.path()).unwrap();
        db.put_cell(CellId(1), b"title=unrelated\n\nrunbook".to_vec())
            .unwrap();
        db.put_cell(
            CellId(2),
            b"path=confluence/payments/runbook\n\ncommon body".to_vec(),
        )
        .unwrap();

        let view = test_view([RetrievalMode::Balanced]);
        let results = db
            .retrieve_aql(
                r#"RETRIEVE CONTEXT FOR TASK "runbook" IN BRAIN default LIMIT 2 CANDIDATES;"#,
                &view,
            )
            .unwrap();

        assert_eq!(results[0].cell_id, CellId(2));
    }

    #[test]
    fn retrieve_aql_expands_child_hit_with_parent_context() {
        let dir = tempfile::tempdir().unwrap();
        let mut db = Database::open(dir.path()).unwrap();
        db.put_cell(
            CellId(1),
            b"document_id=doc-alpha\nchunk_id=parent-alpha\nchunk_role=parent\ntitle=Alpha parent\n\nParent context includes owner, deadline, and rollout notes."
                .to_vec(),
        )
        .unwrap();
        db.put_cell(
            CellId(2),
            b"document_id=doc-alpha\nchunk_id=child-alpha-1\nparent_id=parent-alpha\nchunk_role=child\nsection=Risk details\n\nspecific-child-anchor appears here."
                .to_vec(),
        )
        .unwrap();

        let view = test_view([RetrievalMode::Balanced]);
        let results = db
            .retrieve_aql(
                r#"RETRIEVE CONTEXT FOR TASK "specific-child-anchor" IN BRAIN default LIMIT 2 CANDIDATES;"#,
                &view,
            )
            .unwrap();

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].cell_id, CellId(2));
        assert_eq!(results[1].cell_id, CellId(1));
    }

    #[test]
    fn operator_executor_matches_direct_retrieve_pipeline_and_reports_trace() {
        let dir = tempfile::tempdir().unwrap();
        let mut db = Database::open(dir.path()).unwrap();
        db.put_cell(
            CellId(1),
            b"document_id=doc-alpha\nchunk_id=parent-alpha\nchunk_role=parent\ncontent_hash=parent\n\nParent context for alpha."
                .to_vec(),
        )
        .unwrap();
        db.put_cell(
            CellId(2),
            b"document_id=doc-alpha\nchunk_id=child-alpha\nparent_id=parent-alpha\nchunk_role=child\ncontent_hash=child\n\nneedle alpha detail"
                .to_vec(),
        )
        .unwrap();
        db.put_cell(
            CellId(3),
            b"content_hash=duplicate\n\nneedle alpha duplicate".to_vec(),
        )
        .unwrap();
        db.put_cell(
            CellId(4),
            b"content_hash=duplicate\ntitle=needle alpha\n\nneedle alpha duplicate".to_vec(),
        )
        .unwrap();

        let view = test_view([RetrievalMode::Balanced]);
        let (cached, index) = db
            .bind_aql_cached(
                r#"RETRIEVE CONTEXT FOR TASK "needle alpha" IN BRAIN default LIMIT 10 CANDIDATES;"#,
                &view,
            )
            .unwrap();
        let BoundPlan::Retrieve(plan) = cached.bound_plan else {
            panic!("expected retrieve plan");
        };
        let provider = EngineAqlProvider::new(index, &view);

        let direct = db.retrieve_cells_direct(&plan, &provider).unwrap();
        let report = db
            .retrieve_cells_with_execution_trace(&plan, &provider)
            .unwrap();

        assert_eq!(report.cells, direct);
        let names = report
            .operators
            .iter()
            .map(|operator| operator.name.as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            names,
            vec![
                "BitmapIndexScan",
                "PermissionFilter",
                "QualityFilter",
                "RankOp",
                "DedupOp",
                "ParentExpandOp",
                "LimitOp",
            ]
        );
        assert!(report.total_elapsed_nanos > 0);
        assert_eq!(
            report
                .operators
                .iter()
                .find(|operator| operator.name == "LimitOp")
                .map(|operator| operator.output_count),
            Some(report.cells.len())
        );
    }

    #[test]
    fn retrieve_aql_suppresses_duplicate_content_hashes() {
        let dir = tempfile::tempdir().unwrap();
        let mut db = Database::open(dir.path()).unwrap();
        db.put_cell(
            CellId(1),
            b"source_hash=source-a\ncontent_hash=same-content\n\nalpha budget duplicate".to_vec(),
        )
        .unwrap();
        db.put_cell(
            CellId(2),
            b"source_hash=source-b\ncontent_hash=same-content\ntitle=alpha budget\n\nalpha budget duplicate".to_vec(),
        )
        .unwrap();

        let view = test_view([RetrievalMode::Balanced]);
        let results = db
            .retrieve_aql(
                r#"RETRIEVE CONTEXT FOR TASK "alpha budget" IN BRAIN default LIMIT 10 CANDIDATES;"#,
                &view,
            )
            .unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].cell_id, CellId(2));
    }

    #[test]
    fn audit_mode_uses_trust_weight_in_retrieval_ordering() {
        let dir = tempfile::tempdir().unwrap();
        let mut db = Database::open(dir.path()).unwrap();
        db.put_cell(
            CellId(1),
            b"source_trust_q16=1000\n\nbudget policy".to_vec(),
        )
        .unwrap();
        db.put_cell(
            CellId(2),
            b"source_trust_q16=65000\n\nbudget policy".to_vec(),
        )
        .unwrap();

        let view = test_view([RetrievalMode::Audit]);
        let results = db
            .retrieve_aql(
                r#"RETRIEVE CONTEXT FOR TASK "budget" IN BRAIN default USING MODE audit LIMIT 2 CANDIDATES;"#,
                &view,
            )
            .unwrap();

        assert_eq!(results[0].cell_id, CellId(2));
    }

    #[test]
    fn semantic_mode_uses_query_vector_for_retrieval_ordering() {
        let dir = tempfile::tempdir().unwrap();
        let mut db = Database::open(dir.path()).unwrap();
        db.put_cell(
            CellId(1),
            b"title=vector alpha\nvector=100,0\n\nshared context".to_vec(),
        )
        .unwrap();
        db.put_cell(
            CellId(2),
            b"title=vector beta\nvector=0,100\n\nshared context".to_vec(),
        )
        .unwrap();

        let view = test_view([RetrievalMode::Semantic]);
        let results = db
            .retrieve_aql(
                r#"RETRIEVE CONTEXT FOR TASK "query_vector=0,100" IN BRAIN default USING MODE semantic LIMIT 2 CANDIDATES;"#,
                &view,
            )
            .unwrap();

        assert_eq!(results[0].cell_id, CellId(2));
    }

    #[test]
    fn semantic_mode_uses_named_view_vectors_for_retrieval_ordering() {
        let dir = tempfile::tempdir().unwrap();
        let mut db = Database::open(dir.path()).unwrap();
        db.put_cell(
            CellId(1),
            b"title=body match\nvector=0,100\n\nshared context".to_vec(),
        )
        .unwrap();
        db.put_cell(
            CellId(2),
            b"title=view match\ntitle_vector=100,0\nvector=0,1\n\nshared context".to_vec(),
        )
        .unwrap();

        let view = test_view([RetrievalMode::Semantic]);
        let results = db
            .retrieve_aql(
                r#"RETRIEVE CONTEXT FOR TASK "query_vector=100,0" IN BRAIN default USING MODE semantic LIMIT 2 CANDIDATES;"#,
                &view,
            )
            .unwrap();

        assert_eq!(results[0].cell_id, CellId(2));
    }

    fn bench_payload(id: u64) -> Vec<u8> {
        format!("scope=bench\nstatus=ready\nsource=bench-{id}\ncell {id} budget ready").into_bytes()
    }

    fn bench_query() -> &'static str {
        r#"RETRIEVE CONTEXT FOR TASK "bench" IN BRAIN default WHERE space = bench AND status = "ready" LIMIT 100 CANDIDATES;"#
    }

    fn bench_view() -> AgentView {
        AgentView {
            agent_id: AgentId(1),
            label: Some("bench".to_owned()),
            readable_brains: BTreeSet::from([BrainId(1)]),
            readable_scopes: BTreeSet::from([scope_id("bench")]),
            writable_scopes: BTreeSet::new(),
            allowed_modes: BTreeSet::from([RetrievalMode::Balanced]),
            allowed_memory_types: BTreeSet::from([MemoryType::Decision]),
            max_context_budget_tokens: 100_000,
            default_context_budget_tokens: 50_000,
            max_candidate_limit: 1000,
            default_candidate_limit: 100,
            min_required_confidence_q16: Q16_ZERO,
            max_ttl_seconds: Some(3_600),
            allow_remember: false,
            allow_verify_fact: false,
            allow_audit_mode: false,
            require_citations_by_default: true,
            private_scope: None,
        }
    }

    #[test]
    #[ignore = "run explicitly: cargo test -p cortex-engine --lib --release operator_executor_overhead -- --ignored --nocapture"]
    fn operator_executor_overhead_within_ten_percent() {
        let dir = tempfile::tempdir().unwrap();
        let mut db = Database::open(dir.path()).unwrap();
        for id in 1..=1000 {
            db.put_cell(CellId(id), bench_payload(id)).unwrap();
        }
        db.checkpoint().unwrap();

        let (cached, index) = db.bind_aql_cached(bench_query(), &bench_view()).unwrap();
        let BoundPlan::Retrieve(plan) = cached.bound_plan else {
            panic!("expected retrieve plan");
        };
        let provider = EngineAqlProvider::new(index, &bench_view());

        let iterations = 100;
        let mut direct_times = Vec::with_capacity(iterations);
        let mut exec_times = Vec::with_capacity(iterations);

        for _ in 0..iterations {
            let start = Instant::now();
            let _ = db.retrieve_cells_direct(&plan, &provider).unwrap();
            direct_times.push(start.elapsed().as_nanos());
        }
        for _ in 0..iterations {
            let start = Instant::now();
            let _ = db
                .retrieve_cells_with_execution_trace(&plan, &provider)
                .unwrap();
            exec_times.push(start.elapsed().as_nanos());
        }

        direct_times.sort();
        exec_times.sort();
        let direct_median = direct_times[iterations / 2];
        let exec_median = exec_times[iterations / 2];
        let ratio = exec_median as f64 / direct_median as f64;
        eprintln!(
            "direct median: {} ns, exec median: {} ns, ratio: {:.3}",
            direct_median, exec_median, ratio
        );
        assert!(
            ratio <= 1.10,
            "executor overhead exceeds 10%: ratio = {ratio}"
        );
    }

    #[test]
    fn pin_read_txn_registers_and_unregisters() {
        let dir = tempfile::tempdir().unwrap();
        let db = Database::open(dir.path()).unwrap();
        let pin = db.pin_read_txn();
        let seq = pin.read_txn().read_seq;
        assert_eq!(db.active_read_pins.lock().unwrap().get(&seq), Some(&1));
        assert_eq!(db.gc_horizon(), seq);
        drop(pin);
        assert!(db.active_read_pins.lock().unwrap().is_empty());
        assert_eq!(db.gc_horizon(), db.current_seq());
    }

    #[test]
    fn gc_horizon_is_oldest_active_pin() {
        let dir = tempfile::tempdir().unwrap();
        let mut db = Database::open(dir.path()).unwrap();
        db.put_cell(CellId(1), b"v1".to_vec()).unwrap();
        let pin1 = db.pin_read_txn();
        let seq1 = pin1.read_txn().read_seq;
        db.put_cell(CellId(2), b"v2".to_vec()).unwrap();
        let pin2 = db.pin_read_txn();
        let seq2 = pin2.read_txn().read_seq;
        db.put_cell(CellId(3), b"v3".to_vec()).unwrap();
        assert_eq!(db.gc_horizon(), seq1);
        drop(pin1);
        assert_eq!(db.gc_horizon(), seq2);
        drop(pin2);
        assert_eq!(db.gc_horizon(), db.current_seq());
    }

    #[test]
    fn pinned_snapshot_preserves_old_version_across_compact() {
        let dir = tempfile::tempdir().unwrap();
        let mut db = Database::open(dir.path()).unwrap();
        db.put_cell(CellId(1), b"v1".to_vec()).unwrap();
        let pin = db.pin_read_txn();
        let txn = pin.read_txn();
        db.patch_cell(CellId(1), b"v2".to_vec()).unwrap();
        db.compact().unwrap();
        assert_eq!(db.get_cell(txn, CellId(1)).unwrap(), b"v1");
        assert_eq!(db.get_latest_cell(CellId(1)).unwrap(), b"v2");
        drop(pin);
        db.compact().unwrap();
        assert_eq!(db.get_latest_cell(CellId(1)).unwrap(), b"v2");
    }
}
