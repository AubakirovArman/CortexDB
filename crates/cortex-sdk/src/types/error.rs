use serde::Deserialize;

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    NotFound,
    BadRequest,
    Unauthorized,
    Forbidden,
    PayloadTooLarge,
    RateLimited,
    QuotaExceeded,
    ServiceUnavailable,
    Internal,
    InvalidAql,
    UnknownField,
    UnsupportedOperator,
    PermissionDenied,
    DatabaseBusy,
    StorageCorruption,
    InvalidTenant,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct ErrorResponse {
    pub code: ErrorCode,
    pub error: String,
    pub message: String,
}
