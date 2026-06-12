mod admin;
mod database_route;
mod error;
mod inference;

use axum::{
    extract::{Request, State},
    http::{header, StatusCode},
    response::{Html, IntoResponse, Response},
    Json,
};
use std::sync::atomic::Ordering;
use std::time::Instant;

use crate::quota::{
    request_allowed_by_principal_quota, request_allowed_by_rate_limit,
    request_body_allowed_by_principal_quota,
};
use crate::request_audit::{audit_http_response, RequestAudit};
use crate::request_id::{request_id_from_headers, with_request_id, RequestIdSource};
use crate::responses::ErrorCode;
use crate::state::AppState;
use crate::{auth, dashboard};

use error::error_response;

pub(crate) async fn axum_handler(State(state): State<AppState>, req: Request) -> Response {
    let request_started = Instant::now();
    let method = req.method().as_str().to_owned();
    let uri = req.uri().to_owned();
    let path = uri.path().to_owned();
    let query = uri.query().unwrap_or("").to_owned();
    let request_id_info = request_id_from_headers(req.headers());
    let request_id = request_id_info.value;
    match request_id_info.source {
        RequestIdSource::Client => {
            state
                .request_id_client_provided
                .fetch_add(1, Ordering::Relaxed);
        }
        RequestIdSource::Generated => {
            state.request_id_generated.fetch_add(1, Ordering::Relaxed);
        }
    }
    let mut audit_event = RequestAudit {
        method: &method,
        path: &path,
        query: &query,
        request_id: &request_id,
        started: request_started,
        principal_id: None,
        auth_role: None,
        auth_agent_id: None,
    };

    let span = tracing::info_span!("http_request", %method, %path, %request_id);
    let _enter = span.enter();

    let auth_header = req
        .headers()
        .get("authorization")
        .or_else(|| req.headers().get("Authorization"))
        .and_then(|h| h.to_str().ok());
    let auth_decision = match auth::authorize_request(&state.options, auth_header, &method, &path) {
        Ok(decision) => decision,
        Err(error) => {
            let status =
                StatusCode::from_u16(error.status_code()).unwrap_or(StatusCode::UNAUTHORIZED);
            audit_http_response(&state, &audit_event, status, Some(error.code()));
            return with_request_id(
                (
                    status,
                    Json(error_response(error.code(), error.to_string())),
                )
                    .into_response(),
                &request_id,
            );
        }
    };

    audit_event.principal_id = auth_decision.principal_id.clone();
    audit_event.auth_role = auth_decision.role.map(auth::AuthRole::as_str);
    audit_event.auth_agent_id = auth_decision.agent_id;

    if let Some(response) = dashboard_response(&state, &method, &path, &audit_event, &request_id) {
        return response;
    }

    let auth_context = auth_decision.route_context();

    if !request_allowed_by_rate_limit(&state) {
        state.request_rejected.fetch_add(1, Ordering::Relaxed);
        return rate_limit_response(
            &state,
            &audit_event,
            &request_id,
            "request rate limit exceeded",
        );
    }

    if !request_allowed_by_principal_quota(&state, &auth_decision) {
        state.request_rejected.fetch_add(1, Ordering::Relaxed);
        return rate_limit_response(
            &state,
            &audit_event,
            &request_id,
            "principal request quota exceeded",
        );
    }

    let body_bytes = match axum::body::to_bytes(req.into_body(), 2 * 1024 * 1024).await {
        Ok(bytes) => bytes.to_vec(),
        Err(_) => {
            audit_http_response(
                &state,
                &audit_event,
                StatusCode::PAYLOAD_TOO_LARGE,
                Some(ErrorCode::PayloadTooLarge),
            );
            return with_request_id(
                (
                    StatusCode::PAYLOAD_TOO_LARGE,
                    Json(error_response(
                        ErrorCode::PayloadTooLarge,
                        "request body exceeds 2MB limit",
                    )),
                )
                    .into_response(),
                &request_id,
            );
        }
    };

    if !request_body_allowed_by_principal_quota(&state, &auth_decision, body_bytes.len()) {
        state.request_rejected.fetch_add(1, Ordering::Relaxed);
        return rate_limit_response(
            &state,
            &audit_event,
            &request_id,
            "principal request body quota exceeded",
        );
    }

    if let Some(response) =
        admin::handle_compactor_control(&state, &method, &path, &audit_event, &request_id)
    {
        return response;
    }
    if let Some(response) = admin::handle_auth_policy_admin(
        &state,
        &method,
        &path,
        &query,
        &body_bytes,
        &audit_event,
        &request_id,
        &auth_decision,
    )
    .await
    {
        return response;
    }
    if let Some(response) = admin::handle_scope_admin(
        &state,
        &method,
        &path,
        &body_bytes,
        &audit_event,
        &request_id,
        &auth_decision,
    )
    .await
    {
        return response;
    }
    if let Some(response) = inference::handle_inference(
        &state,
        &method,
        &path,
        &body_bytes,
        &audit_event,
        &request_id,
    ) {
        return response;
    }

    database_route::handle_database_route(
        &state,
        &method,
        &path,
        &query,
        &body_bytes,
        &audit_event,
        &request_id,
        auth_context,
        &auth_decision,
    )
    .await
}

fn dashboard_response(
    state: &AppState,
    method: &str,
    path: &str,
    audit_event: &RequestAudit<'_>,
    request_id: &str,
) -> Option<Response> {
    if state.options.dashboard_enabled && method == "GET" && dashboard::is_page(path) {
        audit_http_response(state, audit_event, StatusCode::OK, None);
        return Some(with_request_id(
            Html(dashboard::html()).into_response(),
            request_id,
        ));
    }
    if state.options.dashboard_enabled && method == "GET" {
        if let Some(asset) = dashboard::asset(path) {
            audit_http_response(state, audit_event, StatusCode::OK, None);
            return Some(with_request_id(
                (
                    StatusCode::OK,
                    [(header::CONTENT_TYPE, asset.content_type)],
                    asset.body,
                )
                    .into_response(),
                request_id,
            ));
        }
    }
    None
}

fn rate_limit_response(
    state: &AppState,
    audit_event: &RequestAudit<'_>,
    request_id: &str,
    message: &'static str,
) -> Response {
    audit_http_response(
        state,
        audit_event,
        StatusCode::TOO_MANY_REQUESTS,
        Some(ErrorCode::RateLimited),
    );
    with_request_id(
        (
            StatusCode::TOO_MANY_REQUESTS,
            Json(error_response(ErrorCode::RateLimited, message)),
        )
            .into_response(),
        request_id,
    )
}
