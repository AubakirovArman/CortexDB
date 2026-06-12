use axum::http::HeaderValue;
use axum::response::Response;
use std::sync::atomic::{AtomicU64, Ordering};

static REQUEST_ID_COUNTER: AtomicU64 = AtomicU64::new(1);

pub(crate) struct RequestIdInfo {
    pub(crate) value: String,
    pub(crate) source: RequestIdSource,
}

pub(crate) enum RequestIdSource {
    Client,
    Generated,
}

pub(crate) fn request_id_from_headers(headers: &axum::http::HeaderMap) -> RequestIdInfo {
    if let Some(value) = headers
        .get("x-request-id")
        .and_then(|value| value.to_str().ok())
        .filter(|value| is_safe_request_id(value))
    {
        return RequestIdInfo {
            value: value.to_owned(),
            source: RequestIdSource::Client,
        };
    }
    RequestIdInfo {
        value: next_request_id(),
        source: RequestIdSource::Generated,
    }
}

fn is_safe_request_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'))
}

fn next_request_id() -> String {
    let id = REQUEST_ID_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("cortexdb-{id}")
}

pub(crate) fn with_request_id(mut response: Response, request_id: &str) -> Response {
    if let Ok(value) = HeaderValue::from_str(request_id) {
        response.headers_mut().insert("x-request-id", value);
    }
    response
}
