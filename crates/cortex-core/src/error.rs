use crate::types::CellId;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum CoreError {
    #[error("cell not found: {0:?}")]
    CellNotFound(CellId),
}

pub type CoreResult<T> = Result<T, CoreError>;
