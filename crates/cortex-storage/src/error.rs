use std::io;

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("io error: {0}")]
    Io(#[from] io::Error),
    #[error("invalid WAL record")]
    InvalidWalRecord,
    #[error("WAL checksum mismatch")]
    WalChecksumMismatch,
    #[error("incomplete WAL tail")]
    IncompleteTail,
    #[error("invalid WAL file header")]
    InvalidWalFileHeader,
    #[error("invalid segment file")]
    InvalidSegmentFile,
    #[error("invalid bitmap index file")]
    InvalidBitmapIndexFile,
    #[error("invalid lexical index file")]
    InvalidLexicalIndexFile,
    #[error("invalid vector index file")]
    InvalidVectorIndexFile,
    #[error("invalid HNSW graph file")]
    InvalidHnswGraphFile,
    #[error("invalid manifest file")]
    InvalidManifestFile,
    #[error("WAL writer is closed")]
    WalWriterClosed,
    #[error("operation is not implemented yet: {0}")]
    NotImplemented(&'static str),
}

pub type StorageResult<T> = Result<T, StorageError>;
