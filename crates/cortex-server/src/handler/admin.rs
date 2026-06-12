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
use crate::responses::{CompactorControlResponse, ErrorCode};
use crate::state::AppState;
use crate::{auth, auth_policy_store, auth_scope_admin};

pub(super) fn handle_compactor_control(
    state: &AppState,
    method: &str,
    path: &str,
    audit_event: &RequestAudit<'_>,
    request_id: &str,
) -> Option<Response> {
    if method != "POST" || !matches!(path, "/v1/admin/compact/pause" | "/v1/admin/compact/resume") {
        return None;
    }

    let paused = path == "/v1/admin/compact/pause";
    state.compaction_paused.store(paused, Ordering::Relaxed);
    audit_http_response(state, audit_event, StatusCode::OK, None);
    Some(with_request_id(
        (
            StatusCode::OK,
            Json(CompactorControlResponse {
                background_enabled: state.options.background_compaction_enabled,
                paused,
                interval_seconds: state.options.background_compaction_interval_seconds.max(1),
            }),
        )
            .into_response(),
        request_id,
    ))
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn handle_auth_policy_admin(
    state: &AppState,
    method: &str,
    path: &str,
    query: &str,
    body_bytes: &[u8],
    audit_event: &RequestAudit<'_>,
    request_id: &str,
    auth_decision: &auth::AuthDecision,
) -> Option<Response> {
    match auth_policy_store::handle_admin_request(&state.options, method, path, query, body_bytes) {
        Ok(Some(admin_response)) => {
            if admin_response.sync_policy_cells {
                let db = match state.get_db("default") {
                    Ok(db) => db,
                    Err(error) => {
                        audit_http_response(
                            state,
                            audit_event,
                            StatusCode::INTERNAL_SERVER_ERROR,
                            Some(ErrorCode::Internal),
                        );
                        return Some(with_request_id(
                            (
                                StatusCode::INTERNAL_SERVER_ERROR,
                                Json(error_response(ErrorCode::Internal, error.to_string())),
                            )
                                .into_response(),
                            request_id,
                        ));
                    }
                };
                let store_json = admin_response.policy_store_json;
                let _queue_permit = match acquire_principal_queue_permit(state, auth_decision) {
                    Ok(permit) => permit,
                    Err(error) => {
                        state.request_rejected.fetch_add(1, Ordering::Relaxed);
                        let status = StatusCode::from_u16(error.status_code())
                            .unwrap_or(StatusCode::TOO_MANY_REQUESTS);
                        audit_http_response(state, audit_event, status, Some(error.code()));
                        return Some(with_request_id(
                            (
                                status,
                                Json(error_response(
                                    error.code(),
                                    "principal queue quota exceeded",
                                )),
                            )
                                .into_response(),
                            request_id,
                        ));
                    }
                };
                let sync_result =
                    tokio::task::spawn_blocking(move || db.sync_auth_policy_store(&store_json))
                        .await;
                match sync_result {
                    Ok(Ok(_)) => {}
                    Ok(Err(error)) => {
                        let status = StatusCode::from_u16(error.status_code())
                            .unwrap_or(StatusCode::BAD_REQUEST);
                        audit_http_response(state, audit_event, status, Some(error.code()));
                        return Some(with_request_id(
                            (
                                status,
                                Json(error_response(error.code(), error.to_string())),
                            )
                                .into_response(),
                            request_id,
                        ));
                    }
                    Err(_) => {
                        audit_http_response(
                            state,
                            audit_event,
                            StatusCode::INTERNAL_SERVER_ERROR,
                            Some(ErrorCode::Internal),
                        );
                        return Some(with_request_id(
                            (
                                StatusCode::INTERNAL_SERVER_ERROR,
                                Json(error_response(
                                    ErrorCode::Internal,
                                    "auth policy cell sync task failed",
                                )),
                            )
                                .into_response(),
                            request_id,
                        ));
                    }
                }
            }
            audit_http_response(state, audit_event, StatusCode::OK, None);
            let body_str = admin_response.body;
            let response =
                if let Ok(json_val) = serde_json::from_str::<serde_json::Value>(&body_str) {
                    (StatusCode::OK, Json(json_val)).into_response()
                } else {
                    (StatusCode::OK, body_str).into_response()
                };
            Some(with_request_id(response, request_id))
        }
        Ok(None) => None,
        Err(error) => {
            let status =
                StatusCode::from_u16(error.status_code()).unwrap_or(StatusCode::BAD_REQUEST);
            audit_http_response(state, audit_event, status, Some(error.code()));
            Some(with_request_id(
                (
                    status,
                    Json(error_response(error.code(), error.to_string())),
                )
                    .into_response(),
                request_id,
            ))
        }
    }
}

pub(super) async fn handle_scope_admin(
    state: &AppState,
    method: &str,
    path: &str,
    body_bytes: &[u8],
    audit_event: &RequestAudit<'_>,
    request_id: &str,
    auth_decision: &auth::AuthDecision,
) -> Option<Response> {
    if method != "POST"
        || !matches!(
            path,
            "/v1/admin/auth/scope/grant" | "/v1/admin/auth/scope/revoke"
        )
    {
        return None;
    }

    let grant = path == "/v1/admin/auth/scope/grant";
    let (agent_id, scope, access) = match auth_scope_admin::parse_scope_mutation_body(body_bytes) {
        Ok(value) => value,
        Err(error) => {
            let status =
                StatusCode::from_u16(error.status_code()).unwrap_or(StatusCode::BAD_REQUEST);
            audit_http_response(state, audit_event, status, Some(error.code()));
            return Some(with_request_id(
                (
                    status,
                    Json(error_response(error.code(), error.to_string())),
                )
                    .into_response(),
                request_id,
            ));
        }
    };
    let db = match state.get_db("default") {
        Ok(db) => db,
        Err(error) => {
            audit_http_response(
                state,
                audit_event,
                StatusCode::INTERNAL_SERVER_ERROR,
                Some(ErrorCode::Internal),
            );
            return Some(with_request_id(
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(error_response(ErrorCode::Internal, error.to_string())),
                )
                    .into_response(),
                request_id,
            ));
        }
    };
    let _queue_permit = match acquire_principal_queue_permit(state, auth_decision) {
        Ok(permit) => permit,
        Err(error) => {
            state.request_rejected.fetch_add(1, Ordering::Relaxed);
            let status =
                StatusCode::from_u16(error.status_code()).unwrap_or(StatusCode::TOO_MANY_REQUESTS);
            audit_http_response(state, audit_event, status, Some(error.code()));
            return Some(with_request_id(
                (
                    status,
                    Json(error_response(
                        error.code(),
                        "principal queue quota exceeded",
                    )),
                )
                    .into_response(),
                request_id,
            ));
        }
    };
    let result =
        tokio::task::spawn_blocking(move || db.mutate_agent_scope(agent_id, &scope, access, grant))
            .await;
    match result {
        Ok(Ok(response)) => {
            audit_http_response(state, audit_event, StatusCode::OK, None);
            Some(with_request_id(
                (StatusCode::OK, Json(response)).into_response(),
                request_id,
            ))
        }
        Ok(Err(error)) => {
            let status =
                StatusCode::from_u16(error.status_code()).unwrap_or(StatusCode::BAD_REQUEST);
            audit_http_response(state, audit_event, status, Some(error.code()));
            Some(with_request_id(
                (
                    status,
                    Json(error_response(error.code(), error.to_string())),
                )
                    .into_response(),
                request_id,
            ))
        }
        Err(_) => {
            audit_http_response(
                state,
                audit_event,
                StatusCode::INTERNAL_SERVER_ERROR,
                Some(ErrorCode::Internal),
            );
            Some(with_request_id(
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(error_response(
                        ErrorCode::Internal,
                        "auth scope mutation task failed",
                    )),
                )
                    .into_response(),
                request_id,
            ))
        }
    }
}
