use std::fs;
use std::path::{Path, PathBuf};

use cortex_aql::{eval_bitmap_program, BitmapProvider, BoundRetrievePlan};
use cortex_core::memtable::{CellVersion, MemTable, ReadTxn};
use cortex_core::{CellId, CommitSeq};
use cortex_storage::manifest::StorageManifest;
use cortex_storage::wal::{DurabilityMode, WalWriter, WalWriterHandle};

use crate::checkpoint::{load_checkpoint, manifest_path, segments_path};
use crate::error::{EngineError, EngineResult};
use crate::operation::{wal_record_from_operation_with_seq, DbOperation};
use crate::replay::{replay_wal_best_effort_into, replay_wal_into};

pub trait CandidateResolver: BitmapProvider {
    fn cell_id_for_candidate(&self, candidate: u32) -> Option<CellId>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecoveryMode {
    Strict,
    BestEffort,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DatabaseOptions {
    pub durability_mode: DurabilityMode,
    pub recovery_mode: RecoveryMode,
}

impl Default for DatabaseOptions {
    fn default() -> Self {
        Self {
            durability_mode: DurabilityMode::Strict,
            recovery_mode: RecoveryMode::Strict,
        }
    }
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
    pub fn open(path: impl AsRef<Path>) -> EngineResult<Self> {
        Self::open_with_options(path, DatabaseOptions::default())
    }

    pub fn open_with_options(
        path: impl AsRef<Path>,
        options: DatabaseOptions,
    ) -> EngineResult<Self> {
        let root_path = path.as_ref().to_owned();
        fs::create_dir_all(&root_path)?;
        let wal_path = root_path.join("db.aclog");
        let manifest_path = manifest_path(&root_path);
        let segments_path = segments_path(&root_path);
        let checkpoint = load_checkpoint(&root_path)?;
        let replay = match options.recovery_mode {
            RecoveryMode::Strict => replay_wal_into(
                &wal_path,
                checkpoint.memtable,
                CommitSeq(checkpoint.manifest.checkpoint_seq),
            )?,
            RecoveryMode::BestEffort => replay_wal_best_effort_into(
                &wal_path,
                checkpoint.memtable,
                CommitSeq(checkpoint.manifest.checkpoint_seq),
            )?,
        };
        truncate_wal_tail(&wal_path, replay.safe_truncate_offset)?;
        let writer = WalWriter::start(&wal_path, options.durability_mode)?;
        Ok(Self {
            root_path,
            wal_path,
            manifest_path,
            segments_path,
            manifest: checkpoint.manifest,
            memtable: replay.memtable,
            writer,
            current_seq: replay.last_seq,
            durability_mode: options.durability_mode,
        })
    }

    pub fn put_cell(&mut self, cell_id: CellId, payload: Vec<u8>) -> EngineResult<CommitSeq> {
        self.append_then_apply(DbOperation::PutCell { cell_id, payload })
    }

    pub fn patch_cell(&mut self, cell_id: CellId, payload: Vec<u8>) -> EngineResult<CommitSeq> {
        self.require_visible_cell(cell_id)?;
        self.append_then_apply(DbOperation::PatchCell { cell_id, payload })
    }

    pub fn tombstone_cell(&mut self, cell_id: CellId) -> EngineResult<CommitSeq> {
        self.require_visible_cell(cell_id)?;
        self.append_then_apply(DbOperation::TombstoneCell { cell_id })
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

    pub fn wal_path(&self) -> &Path {
        &self.wal_path
    }

    pub fn manifest(&self) -> &StorageManifest {
        &self.manifest
    }

    pub fn close(self) {}

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
            DbOperation::TombstoneCell { cell_id } => self
                .memtable
                .tombstone_cell(cell_id, seq)
                .map_err(EngineError::from),
        }
    }

    fn require_visible_cell(&self, cell_id: CellId) -> EngineResult<()> {
        self.memtable
            .read(self.read_txn(), cell_id)
            .map(|_| ())
            .ok_or_else(|| cortex_core::CoreError::CellNotFound(cell_id).into())
    }
}

pub(crate) fn truncate_wal_tail(path: &Path, safe_offset: u64) -> EngineResult<()> {
    match fs::metadata(path) {
        Ok(metadata) if metadata.len() > safe_offset => {
            fs::OpenOptions::new()
                .write(true)
                .open(path)?
                .set_len(safe_offset)?;
        }
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    Ok(())
}
