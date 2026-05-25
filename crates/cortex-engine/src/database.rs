use std::fs;
use std::path::{Path, PathBuf};

use cortex_core::memtable::{MemTable, ReadTxn};
use cortex_core::{CellId, CommitSeq};
use cortex_storage::wal::{DurabilityMode, WalWriter, WalWriterHandle};

use crate::error::{EngineError, EngineResult};
use crate::operation::{wal_record_from_operation, DbOperation};
use crate::replay::replay_wal;

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
    root_path: PathBuf,
    wal_path: PathBuf,
    memtable: MemTable,
    writer: WalWriterHandle,
    current_seq: CommitSeq,
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
        let replay = replay_wal(&wal_path)?;
        truncate_wal_tail(&wal_path, replay.safe_truncate_offset)?;
        let writer = WalWriter::start(&wal_path, options.durability_mode)?;
        Ok(Self {
            root_path,
            wal_path,
            memtable: replay.memtable,
            writer,
            current_seq: replay.last_seq,
        })
    }

    pub fn put_cell(&mut self, cell_id: CellId, payload: Vec<u8>) -> EngineResult<CommitSeq> {
        self.append_then_apply(DbOperation::PutCell { cell_id, payload })
    }

    pub fn patch_cell(&mut self, cell_id: CellId, payload: Vec<u8>) -> EngineResult<CommitSeq> {
        self.append_then_apply(DbOperation::PatchCell { cell_id, payload })
    }

    pub fn tombstone_cell(&mut self, cell_id: CellId) -> EngineResult<CommitSeq> {
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

    pub fn current_seq(&self) -> CommitSeq {
        self.current_seq
    }

    pub fn root_path(&self) -> &Path {
        &self.root_path
    }

    pub fn wal_path(&self) -> &Path {
        &self.wal_path
    }

    pub fn close(self) {}

    fn append_then_apply(&mut self, operation: DbOperation) -> EngineResult<CommitSeq> {
        let next_seq = CommitSeq(self.current_seq.0 + 1);
        let record = wal_record_from_operation(&operation);
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
}

fn truncate_wal_tail(path: &Path, safe_offset: u64) -> EngineResult<()> {
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
