use crate::responses::{ErrorCode, RouterError};

#[test]
fn all_router_errors_have_stable_codes_and_statuses() {
    let cases = [
        (
            RouterError::NotFound("missing".to_owned()),
            404,
            ErrorCode::NotFound,
            "not_found",
        ),
        (
            RouterError::BadRequest("bad".to_owned()),
            400,
            ErrorCode::BadRequest,
            "bad_request",
        ),
        (
            RouterError::InvalidAql("invalid".to_owned()),
            400,
            ErrorCode::InvalidAql,
            "invalid_aql",
        ),
        (
            RouterError::PermissionDenied("denied".to_owned()),
            403,
            ErrorCode::PermissionDenied,
            "permission_denied",
        ),
        (
            RouterError::Unauthorized,
            401,
            ErrorCode::Unauthorized,
            "unauthorized",
        ),
        (
            RouterError::Forbidden("forbidden".to_owned()),
            403,
            ErrorCode::Forbidden,
            "forbidden",
        ),
        (
            RouterError::PayloadTooLarge,
            413,
            ErrorCode::PayloadTooLarge,
            "payload_too_large",
        ),
        (
            RouterError::RateLimited,
            429,
            ErrorCode::RateLimited,
            "rate_limited",
        ),
        (
            RouterError::DatabaseBusy("busy".to_owned()),
            503,
            ErrorCode::DatabaseBusy,
            "database_busy",
        ),
        (
            RouterError::ServiceUnavailable,
            503,
            ErrorCode::ServiceUnavailable,
            "service_unavailable",
        ),
        (
            RouterError::StorageCorruption("corrupt".to_owned()),
            500,
            ErrorCode::StorageCorruption,
            "storage_corruption",
        ),
        (
            RouterError::Internal("internal".to_owned()),
            500,
            ErrorCode::Internal,
            "internal",
        ),
    ];

    for (error, status, code, code_str) in cases {
        assert_eq!(error.status_code(), status);
        assert_eq!(error.code(), code);
        assert_eq!(error.code().as_str(), code_str);
    }
}

#[test]
fn non_router_gateway_codes_are_stable() {
    assert_eq!(ErrorCode::InvalidTenant.as_str(), "invalid_tenant");
}
