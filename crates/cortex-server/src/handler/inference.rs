use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};

use super::error::error_response;
use crate::llm;
use crate::request_audit::{audit_http_response, audit_llm_inference_decision, RequestAudit};
use crate::request_id::with_request_id;
use crate::state::AppState;

pub(super) fn handle_inference(
    state: &AppState,
    method: &str,
    path: &str,
    body_bytes: &[u8],
    audit_event: &RequestAudit<'_>,
    request_id: &str,
) -> Option<Response> {
    if method != "POST" || path != "/v1/inference" {
        return None;
    }

    match llm::handle_inference_test_double(body_bytes, state.options.llm_test_double_enabled) {
        Ok(result) => {
            audit_llm_inference_decision(state, audit_event, StatusCode::OK, None, &result.audit);
            audit_http_response(state, audit_event, StatusCode::OK, None);
            let body_str = result.body;
            let response =
                if let Ok(json_val) = serde_json::from_str::<serde_json::Value>(&body_str) {
                    (StatusCode::OK, Json(json_val)).into_response()
                } else {
                    (StatusCode::OK, body_str).into_response()
                };
            Some(with_request_id(response, request_id))
        }
        Err(error) => {
            let status =
                StatusCode::from_u16(error.error.status_code()).unwrap_or(StatusCode::BAD_REQUEST);
            audit_llm_inference_decision(
                state,
                audit_event,
                status,
                Some(error.error.code()),
                &error.audit,
            );
            audit_http_response(state, audit_event, status, Some(error.error.code()));
            Some(with_request_id(
                (
                    status,
                    Json(error_response(error.error.code(), error.error.to_string())),
                )
                    .into_response(),
                request_id,
            ))
        }
    }
}
