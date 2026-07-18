mod admin;
mod agent_admin;
mod database_route;
mod error;
mod inference;

use axum::{
    extract::{Request, State},
    http::{header, HeaderName, HeaderValue, StatusCode},
    response::{Html, IntoResponse, Response},
    Json,
};
use std::sync::atomic::Ordering;
use std::time::Instant;

use crate::quota::{
    request_allowed_by_principal_quota, request_allowed_by_rate_limit,
    request_body_allowed_by_principal_quota,
};
use crate::request_audit::{
    audit_http_response, audit_http_response_with_receipt_hash, RequestAudit,
};
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
        .and_then(|h| h.to_str().ok())
        .map(str::to_owned);
    let content_type = req
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|h| h.to_str().ok())
        .map(str::to_owned);
    let auth_decision =
        match auth::authorize_request(&state.options, auth_header.as_deref(), &method, &path) {
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
        return quota_exceeded_response(
            &state,
            &audit_event,
            &request_id,
            "principal request quota exceeded",
        );
    }

    let target_for_timeout = if query.is_empty() {
        path.clone()
    } else {
        format!("{path}?{query}")
    };
    let body_timeout = std::time::Duration::from_millis(crate::actor::route_timeout_ms(
        &state.options,
        &method,
        &target_for_timeout,
    ));
    let body_bytes = match tokio::time::timeout(
        body_timeout,
        axum::body::to_bytes(req.into_body(), 2 * 1024 * 1024),
    )
    .await
    {
        Ok(Ok(bytes)) => bytes.to_vec(),
        Ok(Err(_)) => {
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
        Err(_) => {
            return request_timeout_response(
                &state,
                &audit_event,
                &request_id,
                "request timed out while reading body",
            );
        }
    };

    if !request_body_allowed_by_principal_quota(&state, &auth_decision, body_bytes.len()) {
        state.request_rejected.fetch_add(1, Ordering::Relaxed);
        return quota_exceeded_response(
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
    if let Some(response) = agent_admin::handle_agent_admin(
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
    if method == "GET" && path == "/v1/cluster/status" {
        audit_http_response(&state, &audit_event, StatusCode::OK, None);
        return with_request_id(
            (
                StatusCode::OK,
                Json(crate::cluster::status_response(&state.options)),
            )
                .into_response(),
            &request_id,
        );
    }
    if let Some(decision) = crate::cluster::context_ingress_decision_with_monitor(
        &state.options,
        state.cluster_ingress_monitor.as_deref(),
        &method,
        &path,
    ) {
        match decision {
            crate::cluster::ContextIngressDecision::Local => {}
            crate::cluster::ContextIngressDecision::Forward(target) => {
                let method_clone = method.clone();
                let target_path = target_for_timeout.clone();
                let body_clone = body_bytes.clone();
                let auth_clone = auth_header.clone();
                let content_type_clone = content_type.clone();
                let request_id_clone = request_id.clone();
                let forward_result = tokio::task::spawn_blocking(move || {
                    crate::cluster::forward_http_request(
                        &target,
                        &method_clone,
                        &target_path,
                        &body_clone,
                        auth_clone.as_deref(),
                        content_type_clone.as_deref(),
                        Some(&request_id_clone),
                    )
                })
                .await
                .unwrap_or_else(|error| {
                    Err(format!("live Raft ingress forwarding task failed: {error}"))
                });
                match forward_result {
                    Ok(response) => {
                        let status = StatusCode::from_u16(response.status_code)
                            .unwrap_or(StatusCode::BAD_GATEWAY);
                        let receipt_hash =
                            crate::receipt::accountability_receipt_audit_hash_from_response_body(
                                &response.body,
                            );
                        audit_http_response_with_receipt_hash(
                            &state,
                            &audit_event,
                            status,
                            None,
                            receipt_hash.as_deref(),
                        );
                        return with_request_id(
                            forwarded_body_response(status, response.body),
                            &request_id,
                        );
                    }
                    Err(message) => {
                        return live_raft_ingress_unavailable_response(
                            &state,
                            &audit_event,
                            &request_id,
                            message,
                        );
                    }
                }
            }
            crate::cluster::ContextIngressDecision::Unavailable(message) => {
                return live_raft_ingress_unavailable_response(
                    &state,
                    &audit_event,
                    &request_id,
                    message,
                );
            }
        }
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

fn forwarded_body_response(status: StatusCode, body: String) -> Response {
    if let Ok(json_val) = serde_json::from_str::<serde_json::Value>(&body) {
        (status, Json(json_val)).into_response()
    } else {
        (status, [(header::CONTENT_TYPE, "text/plain")], body).into_response()
    }
}

fn live_raft_ingress_unavailable_response(
    state: &AppState,
    audit_event: &RequestAudit<'_>,
    request_id: &str,
    message: String,
) -> Response {
    state.request_rejected.fetch_add(1, Ordering::Relaxed);
    audit_http_response(
        state,
        audit_event,
        StatusCode::SERVICE_UNAVAILABLE,
        Some(ErrorCode::ServiceUnavailable),
    );
    with_request_id(
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(error_response(ErrorCode::ServiceUnavailable, message)),
        )
            .into_response(),
        request_id,
    )
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
        let response = dashboard_security_headers(Html(dashboard::html()).into_response(), false);
        return Some(with_request_id(response, request_id));
    }
    if state.options.dashboard_enabled && method == "GET" {
        if let Some(asset) = dashboard::asset(path) {
            audit_http_response(state, audit_event, StatusCode::OK, None);
            let response = (
                StatusCode::OK,
                [(header::CONTENT_TYPE, asset.content_type)],
                asset.body,
            )
                .into_response();
            return Some(with_request_id(
                dashboard_security_headers(response, true),
                request_id,
            ));
        }
    }
    None
}

fn dashboard_security_headers(mut response: Response, immutable_asset: bool) -> Response {
    let headers = response.headers_mut();
    for (name, value) in dashboard::SECURITY_HEADERS {
        headers.insert(
            HeaderName::from_static(name),
            HeaderValue::from_static(value),
        );
    }
    headers.insert(
        header::CACHE_CONTROL,
        if immutable_asset {
            HeaderValue::from_static("public, max-age=31536000, immutable")
        } else {
            HeaderValue::from_static("no-cache")
        },
    );
    response
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

fn quota_exceeded_response(
    state: &AppState,
    audit_event: &RequestAudit<'_>,
    request_id: &str,
    message: &'static str,
) -> Response {
    audit_http_response(
        state,
        audit_event,
        StatusCode::TOO_MANY_REQUESTS,
        Some(ErrorCode::QuotaExceeded),
    );
    with_request_id(
        (
            StatusCode::TOO_MANY_REQUESTS,
            Json(error_response(ErrorCode::QuotaExceeded, message)),
        )
            .into_response(),
        request_id,
    )
}

pub(super) fn request_timeout_response(
    state: &AppState,
    audit_event: &RequestAudit<'_>,
    request_id: &str,
    message: &'static str,
) -> Response {
    state.request_rejected.fetch_add(1, Ordering::Relaxed);
    state.request_timeout.fetch_add(1, Ordering::Relaxed);
    audit_http_response(
        state,
        audit_event,
        StatusCode::SERVICE_UNAVAILABLE,
        Some(ErrorCode::ServiceUnavailable),
    );
    with_request_id(
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(error_response(ErrorCode::ServiceUnavailable, message)),
        )
            .into_response(),
        request_id,
    )
}
