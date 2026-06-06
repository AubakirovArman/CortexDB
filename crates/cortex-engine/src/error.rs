use cortex_aql::BindError;
use cortex_core::{CellId, CoreError};
use cortex_storage::StorageError;
use std::path::PathBuf;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EngineErrorCategory {
    UserInput,
    Permission,
    NotFound,
    Busy,
    Corruption,
    Unavailable,
    Internal,
}

impl EngineErrorCategory {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::UserInput => "user_input",
            Self::Permission => "permission",
            Self::NotFound => "not_found",
            Self::Busy => "busy",
            Self::Corruption => "corruption",
            Self::Unavailable => "unavailable",
            Self::Internal => "internal",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EngineErrorCode {
    BadRequest,
    InvalidAql,
    UnknownField,
    UnsupportedOperator,
    PermissionDenied,
    Forbidden,
    NotFound,
    DatabaseBusy,
    StorageCorruption,
    ServiceUnavailable,
    Internal,
}

impl EngineErrorCode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::BadRequest => "bad_request",
            Self::InvalidAql => "invalid_aql",
            Self::UnknownField => "unknown_field",
            Self::UnsupportedOperator => "unsupported_operator",
            Self::PermissionDenied => "permission_denied",
            Self::Forbidden => "forbidden",
            Self::NotFound => "not_found",
            Self::DatabaseBusy => "database_busy",
            Self::StorageCorruption => "storage_corruption",
            Self::ServiceUnavailable => "service_unavailable",
            Self::Internal => "internal",
        }
    }

    pub fn http_status(self) -> u16 {
        match self {
            Self::BadRequest
            | Self::InvalidAql
            | Self::UnknownField
            | Self::UnsupportedOperator => 400,
            Self::PermissionDenied | Self::Forbidden => 403,
            Self::NotFound => 404,
            Self::StorageCorruption | Self::Internal => 500,
            Self::DatabaseBusy | Self::ServiceUnavailable => 503,
        }
    }

    pub fn category(self) -> EngineErrorCategory {
        match self {
            Self::BadRequest
            | Self::InvalidAql
            | Self::UnknownField
            | Self::UnsupportedOperator => EngineErrorCategory::UserInput,
            Self::PermissionDenied | Self::Forbidden => EngineErrorCategory::Permission,
            Self::NotFound => EngineErrorCategory::NotFound,
            Self::DatabaseBusy => EngineErrorCategory::Busy,
            Self::StorageCorruption => EngineErrorCategory::Corruption,
            Self::ServiceUnavailable => EngineErrorCategory::Unavailable,
            Self::Internal => EngineErrorCategory::Internal,
        }
    }
}

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
    #[error("engine feature is disabled: {0}")]
    FeatureDisabled(&'static str),
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
    #[error("HNSW build config value out of range: {field}={value}")]
    HnswBuildConfigOutOfRange { field: &'static str, value: usize },
    #[error("invalid candidate id: {0}")]
    InvalidCandidateId(u32),
    #[error("database is already open: {0}; if this is a stale lock, close the running process or remove db.lock with cortexdb unlock <path> --force")]
    DatabaseAlreadyOpen(PathBuf),
    #[error("backup or restore target already exists: {0}")]
    BackupTargetExists(PathBuf),
    #[error("not leader: node {local} cannot perform write, leader is {leader:?}")]
    NotLeader { local: u64, leader: Option<u64> },
}

impl EngineError {
    pub fn code(&self) -> EngineErrorCode {
        match self {
            Self::Core(CoreError::CellNotFound(_)) => EngineErrorCode::NotFound,
            Self::AqlParse(_) => EngineErrorCode::InvalidAql,
            Self::AqlBind(BindError::PolicyDenied(_)) => EngineErrorCode::PermissionDenied,
            Self::AqlBind(BindError::FieldNotFilterable(_)) => EngineErrorCode::UnknownField,
            Self::AqlBind(BindError::UnsupportedComparator) => EngineErrorCode::UnsupportedOperator,
            Self::AqlBind(_) => EngineErrorCode::InvalidAql,
            Self::InvalidOperation
            | Self::FeatureDisabled(_)
            | Self::InvalidAnnFixture(_)
            | Self::InvalidAnnCorpus(_)
            | Self::VectorDimensionMismatch { .. }
            | Self::HnswBuildConfigOutOfRange { .. }
            | Self::BackupTargetExists(_) => EngineErrorCode::BadRequest,
            Self::Io(error) if error.kind() == std::io::ErrorKind::NotFound => {
                EngineErrorCode::NotFound
            }
            Self::Io(error) if error.kind() == std::io::ErrorKind::PermissionDenied => {
                EngineErrorCode::Forbidden
            }
            Self::DatabaseAlreadyOpen(_) => EngineErrorCode::DatabaseBusy,
            Self::Storage(_)
            | Self::MissingWalSection(_)
            | Self::MissingCommitSeq
            | Self::FatalCellMissingAfterWal(_)
            | Self::MissingStorageFile(_)
            | Self::StorageInvariant(_)
            | Self::InvalidCandidateId(_) => EngineErrorCode::StorageCorruption,
            Self::NotLeader { .. } => EngineErrorCode::ServiceUnavailable,
            Self::BitmapVm(_) | Self::Io(_) | Self::CandidateIdOverflow => {
                EngineErrorCode::Internal
            }
        }
    }

    pub fn category(&self) -> EngineErrorCategory {
        self.code().category()
    }

    pub fn http_status(&self) -> u16 {
        self.code().http_status()
    }

    pub fn safe_message(&self) -> String {
        match self {
            Self::AqlBind(error) => error.safe_message().to_owned(),
            _ => self.to_string(),
        }
    }

    pub fn cli_hint(&self) -> Option<&'static str> {
        match self.code() {
            EngineErrorCode::BadRequest => Some("check command arguments and input format"),
            EngineErrorCode::InvalidAql => Some("check AQL syntax in docs/AQL.md"),
            EngineErrorCode::UnknownField => Some("use a filterable AQL field from docs/AQL.md"),
            EngineErrorCode::UnsupportedOperator => {
                Some("use a supported AQL operator for this field")
            }
            EngineErrorCode::PermissionDenied | EngineErrorCode::Forbidden => {
                Some("check auth token, AgentView, scope, and mode permissions")
            }
            EngineErrorCode::NotFound => Some("check that the database path or resource exists"),
            EngineErrorCode::DatabaseBusy => {
                Some("retry later or run cortexdb unlock <path> --force for a stale lock")
            }
            EngineErrorCode::StorageCorruption => {
                Some("run cortexdb validate, then cortexdb repair <path> if needed")
            }
            EngineErrorCode::ServiceUnavailable => {
                Some("retry after the leader or service becomes available")
            }
            EngineErrorCode::Internal => None,
        }
    }
}

pub type EngineResult<T> = Result<T, EngineError>;
