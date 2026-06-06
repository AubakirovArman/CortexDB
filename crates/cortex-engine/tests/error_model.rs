use std::io;
use std::path::PathBuf;

use cortex_aql::{BindError, PolicyError};
use cortex_core::{CellId, CoreError};
use cortex_engine::{EngineError, EngineErrorCategory, EngineErrorCode};
use cortex_storage::StorageError;

#[test]
fn engine_error_codes_categories_and_statuses_are_stable() {
    let cases = [
        (
            EngineError::Core(CoreError::CellNotFound(CellId(1))),
            EngineErrorCode::NotFound,
            EngineErrorCategory::NotFound,
            404,
        ),
        (
            EngineError::AqlParse("bad query".to_owned()),
            EngineErrorCode::InvalidAql,
            EngineErrorCategory::UserInput,
            400,
        ),
        (
            EngineError::AqlBind(BindError::PolicyDenied(PolicyError::ScopeNotReadable)),
            EngineErrorCode::PermissionDenied,
            EngineErrorCategory::Permission,
            403,
        ),
        (
            EngineError::AqlBind(BindError::UnsupportedComparator),
            EngineErrorCode::InvalidAql,
            EngineErrorCategory::UserInput,
            400,
        ),
        (
            EngineError::InvalidOperation,
            EngineErrorCode::BadRequest,
            EngineErrorCategory::UserInput,
            400,
        ),
        (
            EngineError::Io(io::Error::new(io::ErrorKind::PermissionDenied, "no access")),
            EngineErrorCode::Forbidden,
            EngineErrorCategory::Permission,
            403,
        ),
        (
            EngineError::Storage(StorageError::InvalidManifestFile),
            EngineErrorCode::StorageCorruption,
            EngineErrorCategory::Corruption,
            500,
        ),
        (
            EngineError::DatabaseAlreadyOpen(PathBuf::from("db")),
            EngineErrorCode::DatabaseBusy,
            EngineErrorCategory::Busy,
            503,
        ),
        (
            EngineError::NotLeader {
                local: 1,
                leader: Some(2),
            },
            EngineErrorCode::ServiceUnavailable,
            EngineErrorCategory::Unavailable,
            503,
        ),
    ];

    for (error, code, category, http_status) in cases {
        assert_eq!(error.code(), code);
        assert_eq!(error.category(), category);
        assert_eq!(error.http_status(), http_status);
        assert_eq!(error.code().as_str(), code.as_str());
        assert_eq!(error.category().as_str(), category.as_str());
    }
}

#[test]
fn engine_error_safe_messages_and_cli_hints_are_stable() {
    let denied = EngineError::AqlBind(BindError::PolicyDenied(PolicyError::ScopeNotReadable));
    assert_eq!(denied.safe_message(), "requested scope is not readable");
    assert!(denied.cli_hint().unwrap().contains("scope"));

    let corrupt = EngineError::MissingCommitSeq;
    assert_eq!(corrupt.code(), EngineErrorCode::StorageCorruption);
    assert!(corrupt.cli_hint().unwrap().contains("repair"));

    let internal = EngineError::CandidateIdOverflow;
    assert_eq!(internal.code(), EngineErrorCode::Internal);
    assert_eq!(internal.cli_hint(), None);
}
