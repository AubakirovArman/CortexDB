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
    #[error("cell not found after WAL append: {0:?}")]
    FatalCellMissingAfterWal(CellId),
}

pub type EngineResult<T> = Result<T, EngineError>;
