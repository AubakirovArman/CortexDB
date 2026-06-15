use crate::responses::{ErrorCode, ErrorResponse};

#[test]
fn snapshot_all_sdk_visible_error_responses() {
    let responses = [
        ErrorCode::BadRequest,
        ErrorCode::InvalidTenant,
        ErrorCode::InvalidAql,
        ErrorCode::UnknownField,
        ErrorCode::UnsupportedOperator,
        ErrorCode::Unauthorized,
        ErrorCode::Forbidden,
        ErrorCode::PermissionDenied,
        ErrorCode::NotFound,
        ErrorCode::PayloadTooLarge,
        ErrorCode::RateLimited,
        ErrorCode::QuotaExceeded,
        ErrorCode::StorageCorruption,
        ErrorCode::Internal,
        ErrorCode::DatabaseBusy,
        ErrorCode::ServiceUnavailable,
    ]
    .into_iter()
    .map(|code| {
        let code_str = code.as_str();
        ErrorResponse {
            code,
            error: code_str.to_owned(),
            message: format!("{code_str} message"),
        }
    })
    .collect::<Vec<_>>();

    insta::assert_json_snapshot!(responses);
}
