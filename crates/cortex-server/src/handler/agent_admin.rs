use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use std::sync::atomic::Ordering;
use std::sync::Arc;

use super::error::error_response;
use crate::quota::acquire_principal_queue_permit;
use crate::request_audit::{audit_http_response, RequestAudit};
use crate::request_id::with_request_id;
use crate::responses::{ErrorCode, RouterError};
use crate::state::AppState;
use crate::{auth, auth_agent_admin};

#[allow(clippy::too_many_arguments)]
pub(super) async fn handle_agent_admin(
    state: &AppState,
    method: &str,
    path: &str,
    body_bytes: &[u8],
    audit_event: &RequestAudit<'_>,
    request_id: &str,
    auth_decision: &auth::AuthDecision,
) -> Option<Response> {
    if path != "/v1/agents" && !path.starts_with("/v1/agents/") {
        return None;
    }

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

    match agent_admin_operation(db, method, path, body_bytes).await {
        Ok(response) => {
            audit_http_response(state, audit_event, StatusCode::OK, None);
            Some(with_request_id(response, request_id))
        }
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

async fn agent_admin_operation(
    db: Arc<crate::actor::DatabaseActor>,
    method: &str,
    path: &str,
    body_bytes: &[u8],
) -> Result<Response, RouterError> {
    if method == "POST" && path == "/v1/agents" {
        let request = auth_agent_admin::parse_create_body(body_bytes)?;
        let result = tokio::task::spawn_blocking(move || db.create_agent_view(request))
            .await
            .map_err(|_| RouterError::Internal("agent view create task failed".to_owned()))??;
        return Ok((StatusCode::OK, Json(result)).into_response());
    }
    if method == "GET" && path == "/v1/agents" {
        let result = tokio::task::spawn_blocking(move || db.list_agent_views())
            .await
            .map_err(|_| RouterError::Internal("agent view list task failed".to_owned()))??;
        return Ok((StatusCode::OK, Json(result)).into_response());
    }
    if method == "GET" {
        if let Some(agent_id) = auth_agent_admin::parse_agent_path(path) {
            let result = tokio::task::spawn_blocking(move || db.show_agent_view(agent_id))
                .await
                .map_err(|_| RouterError::Internal("agent view show task failed".to_owned()))??;
            return Ok((StatusCode::OK, Json(result)).into_response());
        }
    }
    Err(RouterError::NotFound(
        "unknown agent admin route".to_owned(),
    ))
}
