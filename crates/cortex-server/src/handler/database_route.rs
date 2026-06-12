use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use std::sync::atomic::Ordering;

use super::error::error_response;
use crate::quota::acquire_principal_queue_permit;
use crate::request_audit::{audit_http_response, RequestAudit};
use crate::request_id::with_request_id;
use crate::responses::{ErrorCode, RouterError};
use crate::router::query_param_opt_decoded;
use crate::state::{validate_tenant_id, AppState};
use crate::{auth, http_metrics};

#[allow(clippy::too_many_arguments)]
pub(super) async fn handle_database_route(
    state: &AppState,
    method: &str,
    path: &str,
    query: &str,
    body_bytes: &[u8],
    audit_event: &RequestAudit<'_>,
    request_id: &str,
    auth_context: auth::AuthRouteContext,
    auth_decision: &auth::AuthDecision,
) -> Response {
    let tenant = query_param_opt_decoded(query, "tenant").unwrap_or_else(|| "default".to_owned());
    if !validate_tenant_id(&tenant) {
        audit_http_response(
            state,
            audit_event,
            StatusCode::BAD_REQUEST,
            Some(ErrorCode::InvalidTenant),
        );
        return with_request_id((
            StatusCode::BAD_REQUEST,
            Json(error_response(
                ErrorCode::InvalidTenant,
                "invalid tenant ID structure. Only alphanumeric, '_', and '-' up to 64 characters are allowed.",
            )),
        )
            .into_response(), request_id);
    }
    if !auth::tenant_can_access(auth_decision, &tenant) {
        audit_http_response(
            state,
            audit_event,
            StatusCode::FORBIDDEN,
            Some(ErrorCode::Forbidden),
        );
        return with_request_id(
            (
                StatusCode::FORBIDDEN,
                Json(error_response(
                    ErrorCode::Forbidden,
                    "token tenant policy is not allowed to access this tenant",
                )),
            )
                .into_response(),
            request_id,
        );
    }
    let db = match state.get_db(&tenant) {
        Ok(db) => db,
        Err(e) => {
            audit_http_response(
                state,
                audit_event,
                StatusCode::INTERNAL_SERVER_ERROR,
                Some(ErrorCode::Internal),
            );
            return with_request_id(
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(error_response(ErrorCode::Internal, e.to_string())),
                )
                    .into_response(),
                request_id,
            );
        }
    };

    let target = if query.is_empty() {
        path.to_owned()
    } else {
        format!("{path}?{query}")
    };
    let _queue_permit = match acquire_principal_queue_permit(state, auth_decision) {
        Ok(permit) => permit,
        Err(error) => {
            state.request_rejected.fetch_add(1, Ordering::Relaxed);
            let status =
                StatusCode::from_u16(error.status_code()).unwrap_or(StatusCode::TOO_MANY_REQUESTS);
            audit_http_response(state, audit_event, status, Some(error.code()));
            return with_request_id(
                (
                    status,
                    Json(error_response(
                        error.code(),
                        "principal queue quota exceeded",
                    )),
                )
                    .into_response(),
                request_id,
            );
        }
    };
    let start = std::time::Instant::now();
    let actor = db.clone();
    let method_clone = method.to_owned();
    let target_clone = target.clone();
    let body_clone = body_bytes.to_vec();

    let res = match tokio::task::spawn_blocking(move || {
        actor.route_with_auth(&method_clone, &target_clone, &body_clone, auth_context)
    })
    .await
    {
        Ok(r) => r,
        Err(_) => Err(RouterError::Internal("internal server error".to_owned())),
    };
    if matches!(
        res,
        Err(RouterError::DatabaseBusy(_) | RouterError::ServiceUnavailable)
    ) {
        state.request_rejected.fetch_add(1, Ordering::Relaxed);
    }
    let duration = start.elapsed();
    let duration_ms = duration.as_millis() as u64;
    state.request_count.fetch_add(1, Ordering::Relaxed);
    state
        .request_duration_ms_total
        .fetch_add(duration_ms, Ordering::Relaxed);
    if duration_ms > 50 {
        eprintln!(
            "⚠️ [SLOW QUERY ALERT] method={} target={} took={:?}",
            method, target, duration
        );
    }

    match res {
        Ok(body_str) => {
            audit_http_response(state, audit_event, StatusCode::OK, None);
            if method == "POST"
                && matches!(path, "/v1/search" | "/v1/search/ann-evaluate")
                && http_metrics::record_ann_search_metrics(state, &body_str)
            {
                state.ann_search_latency_ms.observe_ms(duration_ms);
            }
            if method == "GET" && path == "/v1/validate" {
                http_metrics::record_validation_metrics(state, &body_str);
            }
            if method == "GET" && path == "/v1/metrics" {
                if let Some(response) =
                    http_metrics::metrics_response(state, &db, query, &body_str, request_id)
                {
                    return response;
                }
            }
            let response =
                if let Ok(json_val) = serde_json::from_str::<serde_json::Value>(&body_str) {
                    (StatusCode::OK, Json(json_val)).into_response()
                } else {
                    (StatusCode::OK, body_str).into_response()
                };
            with_request_id(response, request_id)
        }
        Err(err) => {
            let status = StatusCode::from_u16(err.status_code()).unwrap_or(StatusCode::BAD_REQUEST);
            audit_http_response(state, audit_event, status, Some(err.code()));
            with_request_id(
                (status, Json(error_response(err.code(), err.to_string()))).into_response(),
                request_id,
            )
        }
    }
}
