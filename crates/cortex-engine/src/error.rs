use cortex_core::{CellId, CoreError};
use cortex_storage::StorageError;
use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    #[error("core error: {0}")]
    Core(#[from] CoreError),
    #[error("storage error: {0}")]
    Storage(#[from] StorageError),
    #[error("bitmap VM error: {0}")]
    BitmapVm(#[from] cortex_aql::BitmapVmError),
    #[error("AQL parse error: {0}")]
    AqlParse(String),
    #[error("AQL bind error: {0}")]
    AqlBind(#[from] cortex_aql::BindError),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid database operation")]
    InvalidOperation,
    #[error("missing WAL section: {0}")]
    MissingWalSection(&'static str),
    #[error("missing WAL commit sequence")]
    MissingCommitSeq,
    #[error("cell not found after WAL append: {0:?}")]
    FatalCellMissingAfterWal(CellId),
    #[error("missing storage file: {0}")]
    MissingStorageFile(PathBuf),
    #[error("storage invariant violation: {0}")]
    StorageInvariant(String),
    #[error("invalid ANN fixture: {0}")]
    InvalidAnnFixture(String),
    #[error("invalid ANN corpus: {0}")]
    InvalidAnnCorpus(String),
    #[error("candidate id overflow")]
    CandidateIdOverflow,
    #[error("vector dimension mismatch: expected {expected}, got {actual}")]
    VectorDimensionMismatch { expected: usize, actual: usize },
    #[error("invalid candidate id: {0}")]
    InvalidCandidateId(u32),
    #[error("database is already open: {0}; if this is a stale lock, close the running process or remove db.lock with cortexdb unlock <path> --force")]
    DatabaseAlreadyOpen(PathBuf),
    #[error("not leader: node {local} cannot perform write, leader is {leader:?}")]
    NotLeader { local: u64, leader: Option<u64> },
}

pub type EngineResult<T> = Result<T, EngineError>;
