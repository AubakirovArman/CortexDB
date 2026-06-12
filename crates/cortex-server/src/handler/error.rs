use crate::responses::{ErrorCode, ErrorResponse};

pub(super) fn error_response(code: ErrorCode, message: impl Into<String>) -> ErrorResponse {
    ErrorResponse {
        code,
        error: code.as_str().to_owned(),
        message: message.into(),
    }
}
