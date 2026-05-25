use cortex_core::{CellId, CoreError};
use cortex_storage::StorageError;

#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    #[error("core error: {0}")]
    Core(#[from] CoreError),
    #[error("storage error: {0}")]
    Storage(#[from] StorageError),
    #[error("bitmap VM error: {0}")]
    BitmapVm(#[from] cortex_aql::BitmapVmError),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid database operation")]
    InvalidOperation,
    #[error("missing WAL section: {0}")]
    MissingWalSection(&'static str),
    #[error("cell not found after WAL append: {0:?}")]
    FatalCellMissingAfterWal(CellId),
}

pub type EngineResult<T> = Result<T, EngineError>;
