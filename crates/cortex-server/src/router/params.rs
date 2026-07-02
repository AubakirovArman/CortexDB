use cortex_core::CellId;

use crate::responses::{ErrorCode, ErrorResponse};

pub fn query_param_opt<'a>(query: &'a str, key: &str) -> Option<&'a str> {
    let prefix = format!("{key}=");
    query.split('&').find_map(|pair| pair.strip_prefix(&prefix))
}

/// Returns the percent-decoded value of a query parameter.
/// Falls back to the raw value if decoding fails.
pub fn query_param_decoded(query: &str, key: &str) -> Result<String, String> {
    let raw = query_param(query, key)?;
    decode_percent(raw)
}

/// Returns the percent-decoded value of an optional query parameter.
/// Falls back to the raw value if decoding fails.
pub fn query_param_opt_decoded(query: &str, key: &str) -> Option<String> {
    query_param_opt(query, key).and_then(|raw| decode_percent(raw).ok())
}

/// Parses an optional non-negative integer query parameter. Returns `None` when
/// the parameter is absent, and a caller-facing error when it is present but not
/// a valid `usize`.
pub fn query_param_usize(query: &str, key: &str) -> Result<Option<usize>, String> {
    match query_param_opt_decoded(query, key) {
        Some(raw) => raw
            .trim()
            .parse::<usize>()
            .map(Some)
            .map_err(|_| format!("{key} must be a non-negative integer")),
        None => Ok(None),
    }
}

fn decode_percent(raw: &str) -> Result<String, String> {
    // Replace '+' with space for application/x-www-form-urlencoded compatibility
    let normalized = raw.replace('+', " ");
    percent_encoding::percent_decode_str(&normalized)
        .decode_utf8()
        .map(|cow| cow.into_owned())
        .map_err(|e| format!("invalid percent-encoding: {e}"))
}

pub fn cell_id(query: &str) -> Result<CellId, String> {
    query_param(query, "cell_id")?
        .parse::<u64>()
        .map(CellId)
        .map_err(|_| "cell_id must be u64".to_owned())
}

pub fn query_param<'a>(query: &'a str, key: &str) -> Result<&'a str, String> {
    let prefix = format!("{key}=");
    query
        .split('&')
        .find_map(|pair| pair.strip_prefix(&prefix))
        .ok_or_else(|| format!("missing {key}"))
}

pub fn json_response(status: u16, body: &str) -> String {
    let reason = reason(status);
    format!(
        "HTTP/1.1 {status} {reason}\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\r\n{body}",
        body.len()
    )
}

pub fn json_error(status: u16, code: ErrorCode, message: &str) -> String {
    let body = serde_json::to_string(&ErrorResponse {
        code,
        error: code.as_str().to_owned(),
        message: message.to_owned(),
    })
    .unwrap_or_else(|_| {
        r#"{"code":"internal","error":"internal","message":"serialization failed"}"#.to_owned()
    });
    json_response(status, &body)
}

pub fn reason(status: u16) -> &'static str {
    match status {
        200 => "OK",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        413 => "Payload Too Large",
        503 => "Service Unavailable",
        500 => "Internal Error",
        _ => "Bad Request",
    }
}
