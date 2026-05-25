use std::io;

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("io error: {0}")]
    Io(#[from] io::Error),
    #[error("invalid WAL record")]
    InvalidWalRecord,
    #[error("WAL checksum mismatch")]
    WalChecksumMismatch,
    #[error("invalid WAL file header")]
    InvalidWalFileHeader,
    #[error("WAL writer is closed")]
    WalWriterClosed,
    #[error("operation is not implemented yet: {0}")]
    NotImplemented(&'static str),
}

pub type StorageResult<T> = Result<T, StorageError>;
