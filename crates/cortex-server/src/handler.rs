use axum::{
    extract::{Request, State},
    http::{header, StatusCode},
    response::{Html, IntoResponse, Response},
    Json,
};
use std::sync::atomic::Ordering;
use std::time::Instant;

use crate::quota::{
    acquire_principal_queue_permit, request_allowed_by_principal_quota,
    request_allowed_by_rate_limit, request_body_allowed_by_principal_quota,
};
use crate::request_audit::{audit_http_response, audit_llm_inference_decision, RequestAudit};
use crate::request_id::{request_id_from_headers, with_request_id, RequestIdSource};
use crate::responses::{CompactorControlResponse, ErrorCode, ErrorResponse, RouterError};
use crate::router::query_param_opt_decoded;
use crate::state::{validate_tenant_id, AppState};
use crate::{auth, auth_policy_store, auth_scope_admin, dashboard, http_metrics, llm};

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

    if state.options.dashboard_enabled && method == "GET" && dashboard::is_page(&path) {
        audit_http_response(&state, &audit_event, StatusCode::OK, None);
        return with_request_id(Html(dashboard::html()).into_response(), &request_id);
    }
    if state.options.dashboard_enabled && method == "GET" {
        if let Some(asset) = dashboard::asset(&path) {
            audit_http_response(&state, &audit_event, StatusCode::OK, None);
            return with_request_id(
                (
                    StatusCode::OK,
                    [(header::CONTENT_TYPE, asset.content_type)],
                    asset.body,
                )
                    .into_response(),
                &request_id,
            );
        }
    }

    let auth_context = auth_decision.route_context();

    if !request_allowed_by_rate_limit(&state) {
        state.request_rejected.fetch_add(1, Ordering::Relaxed);
        audit_http_response(
            &state,
            &audit_event,
            StatusCode::TOO_MANY_REQUESTS,
            Some(ErrorCode::RateLimited),
        );
        return with_request_id(
            (
                StatusCode::TOO_MANY_REQUESTS,
                Json(error_response(
                    ErrorCode::RateLimited,
                    "request rate limit exceeded",
                )),
            )
                .into_response(),
            &request_id,
        );
    }

    if !request_allowed_by_principal_quota(&state, &auth_decision) {
        state.request_rejected.fetch_add(1, Ordering::Relaxed);
        audit_http_response(
            &state,
            &audit_event,
            StatusCode::TOO_MANY_REQUESTS,
            Some(ErrorCode::RateLimited),
        );
        return with_request_id(
            (
                StatusCode::TOO_MANY_REQUESTS,
                Json(error_response(
                    ErrorCode::RateLimited,
                    "principal request quota exceeded",
                )),
            )
                .into_response(),
            &request_id,
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
        audit_http_response(
            &state,
            &audit_event,
            StatusCode::TOO_MANY_REQUESTS,
            Some(ErrorCode::RateLimited),
        );
        return with_request_id(
            (
                StatusCode::TOO_MANY_REQUESTS,
                Json(error_response(
                    ErrorCode::RateLimited,
                    "principal request body quota exceeded",
                )),
            )
                .into_response(),
            &request_id,
        );
    }

    if method == "POST"
        && matches!(
            path.as_str(),
            "/v1/admin/compact/pause" | "/v1/admin/compact/resume"
        )
    {
        let paused = path == "/v1/admin/compact/pause";
        state.compaction_paused.store(paused, Ordering::Relaxed);
        audit_http_response(&state, &audit_event, StatusCode::OK, None);
        return with_request_id(
            (
                StatusCode::OK,
                Json(CompactorControlResponse {
                    background_enabled: state.options.background_compaction_enabled,
                    paused,
                    interval_seconds: state.options.background_compaction_interval_seconds.max(1),
                }),
            )
                .into_response(),
            &request_id,
        );
    }

    match auth_policy_store::handle_admin_request(
        &state.options,
        &method,
        &path,
        &query,
        &body_bytes,
    ) {
        Ok(Some(admin_response)) => {
            if admin_response.sync_policy_cells {
                let db = match state.get_db("default") {
                    Ok(db) => db,
                    Err(error) => {
                        audit_http_response(
                            &state,
                            &audit_event,
                            StatusCode::INTERNAL_SERVER_ERROR,
                            Some(ErrorCode::Internal),
                        );
                        return with_request_id(
                            (
                                StatusCode::INTERNAL_SERVER_ERROR,
                                Json(error_response(ErrorCode::Internal, error.to_string())),
                            )
                                .into_response(),
                            &request_id,
                        );
                    }
                };
                let store_json = admin_response.policy_store_json;
                let _queue_permit = match acquire_principal_queue_permit(&state, &auth_decision) {
                    Ok(permit) => permit,
                    Err(error) => {
                        state.request_rejected.fetch_add(1, Ordering::Relaxed);
                        let status = StatusCode::from_u16(error.status_code())
                            .unwrap_or(StatusCode::TOO_MANY_REQUESTS);
                        audit_http_response(&state, &audit_event, status, Some(error.code()));
                        return with_request_id(
                            (
                                status,
                                Json(error_response(
                                    error.code(),
                                    "principal queue quota exceeded",
                                )),
                            )
                                .into_response(),
                            &request_id,
                        );
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
                    Err(_) => {
                        audit_http_response(
                            &state,
                            &audit_event,
                            StatusCode::INTERNAL_SERVER_ERROR,
                            Some(ErrorCode::Internal),
                        );
                        return with_request_id(
                            (
                                StatusCode::INTERNAL_SERVER_ERROR,
                                Json(error_response(
                                    ErrorCode::Internal,
                                    "auth policy cell sync task failed",
                                )),
                            )
                                .into_response(),
                            &request_id,
                        );
                    }
                }
            }
            audit_http_response(&state, &audit_event, StatusCode::OK, None);
            let body_str = admin_response.body;
            let response =
                if let Ok(json_val) = serde_json::from_str::<serde_json::Value>(&body_str) {
                    (StatusCode::OK, Json(json_val)).into_response()
                } else {
                    (StatusCode::OK, body_str).into_response()
                };
            return with_request_id(response, &request_id);
        }
        Ok(None) => {}
        Err(error) => {
            let status =
                StatusCode::from_u16(error.status_code()).unwrap_or(StatusCode::BAD_REQUEST);
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
    }

    if method == "POST"
        && matches!(
            path.as_str(),
            "/v1/admin/auth/scope/grant" | "/v1/admin/auth/scope/revoke"
        )
    {
        let grant = path == "/v1/admin/auth/scope/grant";
        let (agent_id, scope, access) =
            match auth_scope_admin::parse_scope_mutation_body(&body_bytes) {
                Ok(value) => value,
                Err(error) => {
                    let status = StatusCode::from_u16(error.status_code())
                        .unwrap_or(StatusCode::BAD_REQUEST);
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
        let db = match state.get_db("default") {
            Ok(db) => db,
            Err(error) => {
                audit_http_response(
                    &state,
                    &audit_event,
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Some(ErrorCode::Internal),
                );
                return with_request_id(
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(error_response(ErrorCode::Internal, error.to_string())),
                    )
                        .into_response(),
                    &request_id,
                );
            }
        };
        let _queue_permit = match acquire_principal_queue_permit(&state, &auth_decision) {
            Ok(permit) => permit,
            Err(error) => {
                state.request_rejected.fetch_add(1, Ordering::Relaxed);
                let status = StatusCode::from_u16(error.status_code())
                    .unwrap_or(StatusCode::TOO_MANY_REQUESTS);
                audit_http_response(&state, &audit_event, status, Some(error.code()));
                return with_request_id(
                    (
                        status,
                        Json(error_response(
                            error.code(),
                            "principal queue quota exceeded",
                        )),
                    )
                        .into_response(),
                    &request_id,
                );
            }
        };
        let result = tokio::task::spawn_blocking(move || {
            db.mutate_agent_scope(agent_id, &scope, access, grant)
        })
        .await;
        match result {
            Ok(Ok(response)) => {
                audit_http_response(&state, &audit_event, StatusCode::OK, None);
                return with_request_id(
                    (StatusCode::OK, Json(response)).into_response(),
                    &request_id,
                );
            }
            Ok(Err(error)) => {
                let status =
                    StatusCode::from_u16(error.status_code()).unwrap_or(StatusCode::BAD_REQUEST);
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
            Err(_) => {
                audit_http_response(
                    &state,
                    &audit_event,
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Some(ErrorCode::Internal),
                );
                return with_request_id(
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(error_response(
                            ErrorCode::Internal,
                            "auth scope mutation task failed",
                        )),
                    )
                        .into_response(),
                    &request_id,
                );
            }
        }
    }

    if method == "POST" && path == "/v1/inference" {
        match llm::handle_inference_test_double(&body_bytes, state.options.llm_test_double_enabled)
        {
            Ok(result) => {
                audit_llm_inference_decision(
                    &state,
                    &audit_event,
                    StatusCode::OK,
                    None,
                    &result.audit,
                );
                audit_http_response(&state, &audit_event, StatusCode::OK, None);
                let body_str = result.body;
                let response =
                    if let Ok(json_val) = serde_json::from_str::<serde_json::Value>(&body_str) {
                        (StatusCode::OK, Json(json_val)).into_response()
                    } else {
                        (StatusCode::OK, body_str).into_response()
                    };
                return with_request_id(response, &request_id);
            }
            Err(error) => {
                let status = StatusCode::from_u16(error.error.status_code())
                    .unwrap_or(StatusCode::BAD_REQUEST);
                audit_llm_inference_decision(
                    &state,
                    &audit_event,
                    status,
                    Some(error.error.code()),
                    &error.audit,
                );
                audit_http_response(&state, &audit_event, status, Some(error.error.code()));
                return with_request_id(
                    (
                        status,
                        Json(error_response(error.error.code(), error.error.to_string())),
                    )
                        .into_response(),
                    &request_id,
                );
            }
        }
    }

    let tenant = query_param_opt_decoded(&query, "tenant").unwrap_or_else(|| "default".to_owned());
    if !validate_tenant_id(&tenant) {
        audit_http_response(
            &state,
            &audit_event,
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
            .into_response(), &request_id);
    }
    if !auth::tenant_can_access(&auth_decision, &tenant) {
        audit_http_response(
            &state,
            &audit_event,
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
            &request_id,
        );
    }
    let db = match state.get_db(&tenant) {
        Ok(db) => db,
        Err(e) => {
            audit_http_response(
                &state,
                &audit_event,
                StatusCode::INTERNAL_SERVER_ERROR,
                Some(ErrorCode::Internal),
            );
            return with_request_id(
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(error_response(ErrorCode::Internal, e.to_string())),
                )
                    .into_response(),
                &request_id,
            );
        }
    };

    let target = if query.is_empty() {
        path.clone()
    } else {
        format!("{path}?{query}")
    };
    let _queue_permit = match acquire_principal_queue_permit(&state, &auth_decision) {
        Ok(permit) => permit,
        Err(error) => {
            state.request_rejected.fetch_add(1, Ordering::Relaxed);
            let status =
                StatusCode::from_u16(error.status_code()).unwrap_or(StatusCode::TOO_MANY_REQUESTS);
            audit_http_response(&state, &audit_event, status, Some(error.code()));
            return with_request_id(
                (
                    status,
                    Json(error_response(
                        error.code(),
                        "principal queue quota exceeded",
                    )),
                )
                    .into_response(),
                &request_id,
            );
        }
    };
    let start = std::time::Instant::now();
    let actor = db.clone();
    let method_clone = method.clone();
    let target_clone = target.clone();
    let body_clone = body_bytes.clone();

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
            audit_http_response(&state, &audit_event, StatusCode::OK, None);
            if method == "POST"
                && matches!(path.as_str(), "/v1/search" | "/v1/search/ann-evaluate")
                && http_metrics::record_ann_search_metrics(&state, &body_str)
            {
                state.ann_search_latency_ms.observe_ms(duration_ms);
            }
            if method == "GET" && path == "/v1/validate" {
                http_metrics::record_validation_metrics(&state, &body_str);
            }
            if method == "GET" && path == "/v1/metrics" {
                if let Some(response) =
                    http_metrics::metrics_response(&state, &db, &query, &body_str, &request_id)
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
            with_request_id(response, &request_id)
        }
        Err(err) => {
            let status = StatusCode::from_u16(err.status_code()).unwrap_or(StatusCode::BAD_REQUEST);
            audit_http_response(&state, &audit_event, status, Some(err.code()));
            with_request_id(
                (status, Json(error_response(err.code(), err.to_string()))).into_response(),
                &request_id,
            )
        }
    }
}

fn error_response(code: ErrorCode, message: impl Into<String>) -> ErrorResponse {
    ErrorResponse {
        code,
        error: code.as_str().to_owned(),
        message: message.into(),
    }
}
