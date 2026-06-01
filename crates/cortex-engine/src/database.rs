use std::path::{Path, PathBuf};

use cortex_aql::{eval_bitmap_program, BitmapProvider, BoundRetrievePlan};
use cortex_core::memtable::{CellVersion, MemTable, ReadTxn};
use cortex_core::{CellId, CommitSeq};
use cortex_storage::manifest::StorageManifest;
use cortex_storage::wal::{DurabilityMode, WalWriter, WalWriterHandle};

use crate::checkpoint::{load_checkpoint, manifest_path, segments_path};
use crate::cleanup::{cleanup_orphans, remove_lock_file};
use crate::database_files::find_wal_files;
pub(crate) use crate::database_files::truncate_wal_tail;
use crate::error::{EngineError, EngineResult};
use crate::lock::DatabaseLock;
use crate::operation::{
    wal_record_from_operation_with_metadata, wal_record_from_operation_with_seq, DbOperation,
};
use crate::options::{DatabaseOptions, RecoveryMode, StaleLockPolicy};
use crate::replay::{replay_wal_best_effort_into, replay_wal_into};
use crate::search::HnswBuildConfig;

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
    pub(crate) _lock: DatabaseLock,
    closed: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RetrievedCell {
    pub cell_id: CellId,
    pub payload: Vec<u8>,
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

    pub fn retrieve_cells<P: CandidateResolver>(
        &self,
        plan: &BoundRetrievePlan,
        provider: &P,
    ) -> EngineResult<Vec<RetrievedCell>> {
        let candidates = eval_bitmap_program(&plan.bitmap_program, provider)?;
        let txn = self.read_txn();
        Ok(candidates
            .into_iter()
            .filter_map(|candidate| provider.cell_id_for_candidate(candidate))
            .filter_map(|cell_id| {
                self.get_cell(txn, cell_id)
                    .map(|payload| RetrievedCell { cell_id, payload })
            })
            .take(plan.context_policy.candidate_limit as usize)
            .collect())
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
        let record = wal_record_from_operation_with_metadata(next_seq, &operation, metadata);
        self.writer.append(record)?;
        self.apply_operation(next_seq, operation)?;
        self.current_seq = next_seq;
        Ok(next_seq)
    }

    fn apply_operation(&mut self, seq: CommitSeq, operation: DbOperation) -> EngineResult<()> {
        match operation {
            DbOperation::PutCell { cell_id, payload } => {
                self.memtable.put_cell(cell_id, seq, payload);
                Ok(())
            }
            DbOperation::PatchCell { cell_id, payload } => self
                .memtable
                .patch_cell(cell_id, seq, payload)
                .map_err(EngineError::from),
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

impl Drop for Database {
    fn drop(&mut self) {
        if !self.closed {
            let _ = self.writer.shutdown();
        }
    }
}
