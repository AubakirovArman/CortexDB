use axum::{
    extract::{Request, State},
    http::{header, HeaderValue, Method, StatusCode},
    response::{Html, IntoResponse, Response},
    routing::Router,
    Json,
};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tower_http::cors::CorsLayer;
use tower_http::limit::RequestBodyLimitLayer;

use responses::{ErrorCode, ErrorResponse, MetricsResponse};

mod actor;
mod aql;
mod audit;
mod audit_chain;
#[cfg(test)]
mod audit_tests;
mod auth;
mod auth_capability;
mod auth_policy_io;
mod auth_policy_store;
mod authz;
mod context;
mod dashboard;
#[cfg(test)]
mod dashboard_tests;
pub mod external_identity;
mod hnsw_profile;
mod llm;
mod memory;
mod metrics;
mod rate_limit;
pub mod responses;
mod router;
mod search;
#[cfg(test)]
mod search_tests;
#[cfg(test)]
mod sync_handler;
#[cfg(test)]
mod tests;

use crate::rate_limit::{GlobalRateLimit, PrincipalRateLimits};
use crate::responses::RouterError;
pub use auth::{parse_auth_tokens, AuthRole, AuthTokenPolicy};
pub use router::{
    cell_id, json_error, json_response, query_param, query_param_decoded, query_param_opt,
    query_param_opt_decoded, route_database, route_database_with_agent, route_shared,
    route_shared_with_agent,
};
#[cfg(test)]
pub use sync_handler::{handle_http, handle_http_with_options};

pub const DEFAULT_ACTOR_QUEUE_CAPACITY: usize = 1024;
static REQUEST_ID_COUNTER: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ServerOptions {
    pub auth_token: Option<String>,
    /// Optional AgentView id bound to the configured bearer token.
    ///
    /// When set, successful HTTP auth loads the persisted `AgentView` with this
    /// id and scope-bound data routes are checked against that view.
    pub auth_agent_id: Option<u64>,
    /// Additional bearer token policies.
    ///
    /// `AuthRole::Admin` can access all authenticated routes. `AuthRole::Data`
    /// can access data routes and health checks, but not admin/metrics routes.
    /// Each policy may bind to a distinct persisted `AgentView`.
    pub auth_tokens: Vec<AuthTokenPolicy>,
    /// Optional file containing one bearer token policy per line.
    ///
    /// The file uses the same `role:token[:agent_id]` entries as
    /// `CORTEXDB_AUTH_TOKENS`, supports `#` comments and blank lines, and is
    /// re-read for every request so operators can rotate tokens without a
    /// server restart. If the configured file is missing or invalid, auth fails
    /// closed.
    pub auth_tokens_file: Option<PathBuf>,
    /// Optional JSON policy store for durable local auth principals.
    ///
    /// The file is re-read for every request and uses
    /// `schema_version = cortexdb.auth_policy.v1`. Disabled principals are
    /// ignored, invalid stores fail closed, and policies may bind principals to
    /// a role plus optional AgentView id.
    pub auth_policy_store_file: Option<PathBuf>,
    /// Capacity of the bounded actor command queue. Default is 1024.
    pub actor_queue_capacity: usize,
    /// Optional exact browser origin allowed for cross-origin API requests.
    ///
    /// CORS is disabled by default. Set this only for deployments that expose
    /// CortexDB to a browser origin through a trusted reverse proxy.
    pub cors_allowed_origin: Option<String>,
    /// Optional global request limit per 60-second window.
    ///
    /// Rate limiting is disabled by default. This is a coarse Core Alpha guard,
    /// not a replacement for reverse-proxy quotas or user-aware authorization.
    pub request_rate_limit_per_minute: Option<u64>,
    /// Emit structured audit events through `tracing` for HTTP API responses.
    ///
    /// Disabled by default. Audit events intentionally record route/action
    /// metadata, status, tenant, and duration, but not request bodies or query
    /// strings.
    pub audit_log_enabled: bool,
    /// Optional JSONL file sink for audit events.
    ///
    /// If set, audit events are written to this append-only local file and the
    /// file is synced after each event. The sink stores route metadata only,
    /// never request bodies or query strings.
    pub audit_log_path: Option<PathBuf>,
    /// Enables the deterministic local LLM inference test-double endpoint.
    ///
    /// Disabled by default. This does not enable a production model runtime or
    /// external provider calls.
    pub llm_test_double_enabled: bool,
}

impl ServerOptions {
    pub fn actor_queue_capacity(&self) -> usize {
        if self.actor_queue_capacity == 0 {
            DEFAULT_ACTOR_QUEUE_CAPACITY
        } else {
            self.actor_queue_capacity
        }
    }

    pub(crate) fn effective_auth_tokens(&self) -> Vec<AuthTokenPolicy> {
        let mut tokens = self.auth_tokens.clone();
        if let Some(token) = &self.auth_token {
            let mut policy = AuthTokenPolicy::new(token.clone(), AuthRole::Admin);
            policy.agent_id = self.auth_agent_id;
            tokens.push(policy);
        }
        tokens
    }
}

/// Validates that a tenant ID is safe and conforms to the alphanumeric format,
/// preventing path traversal attacks.
///
/// Allowed characters are ASCII alphanumeric, `_`, and `-`. Length must be between 1 and 64.
/// `:` is intentionally disallowed in tenant IDs because a tenant maps directly
/// to a directory name on disk, whereas `:` is reserved for logical scope
/// namespaces (e.g., `project:investments`). This keeps the filesystem
/// boundary clean and prevents accidental scope/tenant collisions.
pub fn validate_tenant_id(tenant: &str) -> bool {
    if tenant.is_empty() || tenant.len() > 64 {
        return false;
    }
    tenant
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

#[derive(Clone)]
pub struct AppState {
    root: PathBuf,
    dbs: Arc<Mutex<BTreeMap<String, Arc<actor::DatabaseActor>>>>,
    options: Arc<ServerOptions>,
    audit_sink: Option<Arc<audit::AuditSink>>,
    request_count: Arc<AtomicU64>,
    request_rejected: Arc<AtomicU64>,
    request_duration_ms_total: Arc<AtomicU64>,
    ann_search_requests: Arc<AtomicU64>,
    ann_fallbacks: Arc<AtomicU64>,
    ann_no_fallback_requests: Arc<AtomicU64>,
    ann_no_fallback_allowed: Arc<AtomicU64>,
    ann_no_fallback_blocked: Arc<AtomicU64>,
    ann_search_latency_ms: metrics::LatencyHistogram,
    validation_failures: Arc<AtomicU64>,
    rate_limit: Option<GlobalRateLimit>,
    principal_rate_limits: PrincipalRateLimits,
}

impl AppState {
    pub fn get_db(&self, tenant: &str) -> std::io::Result<Arc<actor::DatabaseActor>> {
        if !validate_tenant_id(tenant) {
            return Err(std::io::Error::other(format!(
                "invalid tenant id: '{tenant}'"
            )));
        }
        let mut dbs = self
            .dbs
            .lock()
            .map_err(|e| std::io::Error::other(e.to_string()))?;
        if let Some(db) = dbs.get(tenant) {
            return Ok(db.clone());
        }
        let tenant_path = if tenant == "default" {
            self.root.clone()
        } else {
            self.root.join("realms").join(tenant)
        };
        std::fs::create_dir_all(&tenant_path)?;
        let capacity = self.options.actor_queue_capacity();
        let db_shared = Arc::new(actor::DatabaseActor::open_with_capacity(
            &tenant_path,
            capacity,
        )?);
        dbs.insert(tenant.to_owned(), db_shared.clone());
        Ok(db_shared)
    }
}

pub fn serve(root: &Path, addr: &str) -> std::io::Result<()> {
    serve_with_options(root, addr, ServerOptions::default())
}

pub fn serve_with_options(root: &Path, addr: &str, options: ServerOptions) -> std::io::Result<()> {
    validate_server_options(&options)?;
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    rt.block_on(async {
        let cors = cors_layer(&options)?;
        let audit_sink = options
            .audit_log_path
            .as_ref()
            .map(|path| audit::AuditSink::open(path))
            .transpose()?
            .map(Arc::new);
        let rate_limit = options
            .request_rate_limit_per_minute
            .map(GlobalRateLimit::new);
        let state = AppState {
            root: root.to_owned(),
            dbs: Arc::new(Mutex::new(BTreeMap::new())),
            options: Arc::new(options),
            audit_sink,
            request_count: Arc::new(AtomicU64::new(0)),
            request_rejected: Arc::new(AtomicU64::new(0)),
            request_duration_ms_total: Arc::new(AtomicU64::new(0)),
            ann_search_requests: Arc::new(AtomicU64::new(0)),
            ann_fallbacks: Arc::new(AtomicU64::new(0)),
            ann_no_fallback_requests: Arc::new(AtomicU64::new(0)),
            ann_no_fallback_allowed: Arc::new(AtomicU64::new(0)),
            ann_no_fallback_blocked: Arc::new(AtomicU64::new(0)),
            ann_search_latency_ms: metrics::LatencyHistogram::new(),
            validation_failures: Arc::new(AtomicU64::new(0)),
            rate_limit,
            principal_rate_limits: PrincipalRateLimits::default(),
        };

        let mut app = Router::new()
            .fallback(axum_handler)
            .layer(RequestBodyLimitLayer::new(2 * 1024 * 1024)) // 2MB Limit
            .with_state(state.clone());
        if let Some(cors) = cors {
            app = app.layer(cors);
        }

        tokio::spawn(async {
            #[cfg(unix)]
            {
                use tokio::signal::unix::{signal, SignalKind};
                if let Ok(mut stream) = signal(SignalKind::hangup()) {
                    while stream.recv().await.is_some() {
                        println!("♻️ [CONFIG RELOAD SIGHUP] Configuration reloaded successfully without process interruption!");
                    }
                }
            }
            #[cfg(not(unix))]
            {
                tokio::time::sleep(tokio::time::Duration::from_secs(999999)).await;
            }
        });

        // Background TTL expiration task
        let ttl_state = state.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(60));
            loop {
                interval.tick().await;
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                let dbs = match ttl_state.dbs.lock() {
                    Ok(d) => d,
                    Err(_) => continue,
                };
                for actor in dbs.values() {
                    match actor.expire_memory(now) {
                        Ok(expired) if !expired.is_empty() => {
                            tracing::info!(
                                "TTL expiry: {} memory cells tombstoned",
                                expired.len()
                            );
                        }
                        _ => {}
                    }
                }
            }
        });

        let listener = tokio::net::TcpListener::bind(addr).await?;
        axum::serve(listener, app).await?;
        Ok(())
    })
}

fn validate_server_options(options: &ServerOptions) -> std::io::Result<()> {
    if options.auth_agent_id.is_some() && options.auth_token.is_none() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "auth_agent_id requires auth_token",
        ));
    }
    if let Err(error) = auth::validate_token_policies(options) {
        return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, error));
    }
    Ok(())
}

fn cors_layer(options: &ServerOptions) -> std::io::Result<Option<CorsLayer>> {
    let Some(origin) = options.cors_allowed_origin.as_deref() else {
        return Ok(None);
    };
    let origin = HeaderValue::from_str(origin).map_err(|error| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("invalid CORS origin: {error}"),
        )
    })?;
    Ok(Some(
        CorsLayer::new()
            .allow_origin(origin)
            .allow_methods([Method::GET, Method::POST, Method::DELETE, Method::OPTIONS])
            .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE]),
    ))
}

fn request_allowed_by_rate_limit(state: &AppState) -> bool {
    let Some(rate_limit) = &state.rate_limit else {
        return true;
    };
    rate_limit.allow()
}

fn request_allowed_by_principal_quota(state: &AppState, decision: &auth::AuthDecision) -> bool {
    let Some(limit) = decision.request_quota_per_minute else {
        return true;
    };
    let Some(principal_id) = decision.principal_id.as_deref() else {
        return true;
    };
    state.principal_rate_limits.allow(principal_id, limit)
}

fn audit_http_response(
    state: &AppState,
    event: &RequestAudit<'_>,
    status: StatusCode,
    error_code: Option<ErrorCode>,
) {
    if !state.options.audit_log_enabled {
        return;
    }
    let tenant =
        query_param_opt_decoded(event.query, "tenant").unwrap_or_else(|| "default".to_owned());
    audit::emit_http_response(
        audit::HttpResponseAudit {
            method: event.method,
            path: event.path,
            tenant: &tenant,
            request_id: event.request_id,
            principal_id: event.principal_id.as_deref(),
            auth_role: event.auth_role,
            auth_agent_id: event.auth_agent_id,
            status: status.as_u16(),
            error_code: error_code.map(ErrorCode::as_str),
            duration_ms: event.started.elapsed().as_millis() as u64,
        },
        state.audit_sink.as_deref(),
    );
}

fn audit_llm_inference_decision(
    state: &AppState,
    event: &RequestAudit<'_>,
    status: StatusCode,
    error_code: Option<ErrorCode>,
    decision: &llm::LlmInferenceDecisionAudit,
) {
    if !state.options.audit_log_enabled {
        return;
    }
    let tenant =
        query_param_opt_decoded(event.query, "tenant").unwrap_or_else(|| "default".to_owned());
    audit::emit_llm_inference_decision(
        audit::LlmInferenceDecisionAudit {
            tenant: &tenant,
            request_id: event.request_id,
            principal_id: event.principal_id.as_deref(),
            auth_role: event.auth_role,
            auth_agent_id: event.auth_agent_id,
            status: status.as_u16(),
            error_code: error_code.map(ErrorCode::as_str),
            duration_ms: event.started.elapsed().as_millis() as u64,
            outcome: decision.outcome.as_str(),
            reason: decision.reason,
            provider: decision.provider,
            model: decision.model,
            context_cell_count: decision.context_cell_count,
            citation_count: decision.citation_count,
            request_api_key_present: decision.request_api_key_present,
        },
        state.audit_sink.as_deref(),
    );
}

struct RequestAudit<'a> {
    method: &'a str,
    path: &'a str,
    query: &'a str,
    request_id: &'a str,
    started: Instant,
    principal_id: Option<String>,
    auth_role: Option<&'static str>,
    auth_agent_id: Option<u64>,
}

async fn axum_handler(State(state): State<AppState>, req: Request) -> Response {
    let request_started = Instant::now();
    let method = req.method().as_str().to_owned();
    let uri = req.uri().to_owned();
    let path = uri.path().to_owned();
    let query = uri.query().unwrap_or("").to_owned();
    let request_id = request_id_from_headers(req.headers());
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

    if method == "GET" && dashboard::is_page(&path) {
        audit_http_response(&state, &audit_event, StatusCode::OK, None);
        return with_request_id(Html(dashboard::html()).into_response(), &request_id);
    }
    if method == "GET" {
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

    let auth_agent_id = auth_decision.agent_id;

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

    match auth_policy_store::handle_admin_request(
        &state.options,
        &method,
        &path,
        &query,
        &body_bytes,
    ) {
        Ok(Some(body_str)) => {
            audit_http_response(&state, &audit_event, StatusCode::OK, None);
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
    let start = std::time::Instant::now();
    let actor = db.clone();
    let method_clone = method.clone();
    let target_clone = target.clone();
    let body_clone = body_bytes.clone();

    let res = match tokio::task::spawn_blocking(move || {
        actor.route_with_agent(&method_clone, &target_clone, &body_clone, auth_agent_id)
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
                && record_ann_search_metrics(&state, &body_str)
            {
                state.ann_search_latency_ms.observe_ms(duration_ms);
            }
            if method == "GET" && path == "/v1/validate" {
                record_validation_metrics(&state, &body_str);
            }
            if method == "GET" && path == "/v1/metrics" {
                if query.contains("format=prometheus") {
                    let ann_latency = state.ann_search_latency_ms.snapshot();
                    let ann_search_requests = state.ann_search_requests.load(Ordering::Relaxed);
                    let extra = format!(
                        "# HELP cortexdb_actor_queue_depth Current actor command queue depth.\n\
                         # TYPE cortexdb_actor_queue_depth gauge\n\
                         cortexdb_actor_queue_depth {}\n\
                         # HELP cortexdb_actor_queue_capacity Actor command queue capacity.\n\
                         # TYPE cortexdb_actor_queue_capacity gauge\n\
                         cortexdb_actor_queue_capacity {}\n\
                         # HELP cortexdb_request_count Total HTTP requests served.\n\
                         # TYPE cortexdb_request_count counter\n\
                         cortexdb_request_count {}\n\
                         # HELP cortexdb_request_rejected Total HTTP requests rejected due to queue or rate pressure.\n\
                         # TYPE cortexdb_request_rejected counter\n\
                         cortexdb_request_rejected {}\n\
                         # HELP cortexdb_request_duration_ms_total Total HTTP request duration in milliseconds.\n\
                         # TYPE cortexdb_request_duration_ms_total counter\n\
                         cortexdb_request_duration_ms_total {}\n\
                         # HELP cortexdb_ann_search_requests Total ANN-capable search responses observed.\n\
                         # TYPE cortexdb_ann_search_requests counter\n\
                         cortexdb_ann_search_requests {}\n\
                         # HELP cortexdb_ann_fallbacks Total ANN searches that reported fallback.\n\
                         # TYPE cortexdb_ann_fallbacks counter\n\
                         cortexdb_ann_fallbacks {}\n\
                         # HELP cortexdb_ann_no_fallback_requests Total ANN responses with a no-fallback rollout decision.\n\
                         # TYPE cortexdb_ann_no_fallback_requests counter\n\
                         cortexdb_ann_no_fallback_requests {}\n\
                         # HELP cortexdb_ann_no_fallback_allowed Total no-fallback rollout decisions that allowed serving.\n\
                         # TYPE cortexdb_ann_no_fallback_allowed counter\n\
                         cortexdb_ann_no_fallback_allowed {}\n\
                         # HELP cortexdb_ann_no_fallback_blocked Total no-fallback rollout decisions blocked by guardrails.\n\
                         # TYPE cortexdb_ann_no_fallback_blocked counter\n\
                         cortexdb_ann_no_fallback_blocked {}\n\
                         # HELP cortexdb_ann_search_latency_ms ANN-capable HTTP search latency in milliseconds.\n\
                         # TYPE cortexdb_ann_search_latency_ms histogram\n\
                         cortexdb_ann_search_latency_ms_bucket{{le=\"10\"}} {}\n\
                         cortexdb_ann_search_latency_ms_bucket{{le=\"50\"}} {}\n\
                         cortexdb_ann_search_latency_ms_bucket{{le=\"100\"}} {}\n\
                         cortexdb_ann_search_latency_ms_bucket{{le=\"500\"}} {}\n\
                         cortexdb_ann_search_latency_ms_bucket{{le=\"1000\"}} {}\n\
                         cortexdb_ann_search_latency_ms_bucket{{le=\"+Inf\"}} {}\n\
                         cortexdb_ann_search_latency_ms_count {}\n\
                         cortexdb_ann_search_latency_ms_sum {}\n\
                         # HELP cortexdb_validation_failures Total validation responses that reported errors.\n\
                         # TYPE cortexdb_validation_failures counter\n\
                         cortexdb_validation_failures {}\n",
                        db.queue_depth(),
                        db.queue_capacity(),
                        state.request_count.load(Ordering::Relaxed),
                        state.request_rejected.load(Ordering::Relaxed),
                        state.request_duration_ms_total.load(Ordering::Relaxed),
                        ann_search_requests,
                        state.ann_fallbacks.load(Ordering::Relaxed),
                        state.ann_no_fallback_requests.load(Ordering::Relaxed),
                        state.ann_no_fallback_allowed.load(Ordering::Relaxed),
                        state.ann_no_fallback_blocked.load(Ordering::Relaxed),
                        ann_latency.le_10_ms,
                        ann_latency.le_50_ms,
                        ann_latency.le_100_ms,
                        ann_latency.le_500_ms,
                        ann_latency.le_1000_ms,
                        ann_latency.count,
                        ann_latency.count,
                        ann_latency.sum_ms,
                        state.validation_failures.load(Ordering::Relaxed),
                    );
                    return with_request_id(
                        (StatusCode::OK, body_str + &extra).into_response(),
                        &request_id,
                    );
                }
                if let Ok(mut metrics) = serde_json::from_str::<MetricsResponse>(&body_str) {
                    metrics.actor_queue_depth = db.queue_depth();
                    metrics.actor_queue_capacity = db.queue_capacity();
                    metrics.request_count = state.request_count.load(Ordering::Relaxed);
                    metrics.request_rejected = state.request_rejected.load(Ordering::Relaxed);
                    metrics.request_duration_ms_total =
                        state.request_duration_ms_total.load(Ordering::Relaxed);
                    metrics.ann_search_requests = state.ann_search_requests.load(Ordering::Relaxed);
                    metrics.ann_fallbacks = state.ann_fallbacks.load(Ordering::Relaxed);
                    metrics.ann_no_fallback_requests =
                        state.ann_no_fallback_requests.load(Ordering::Relaxed);
                    metrics.ann_no_fallback_allowed =
                        state.ann_no_fallback_allowed.load(Ordering::Relaxed);
                    metrics.ann_no_fallback_blocked =
                        state.ann_no_fallback_blocked.load(Ordering::Relaxed);
                    metrics.ann_search_latency_ms = state.ann_search_latency_ms.snapshot();
                    metrics.validation_failures = state.validation_failures.load(Ordering::Relaxed);
                    return with_request_id(
                        (StatusCode::OK, Json(metrics)).into_response(),
                        &request_id,
                    );
                }
                if let Ok(json_val) = serde_json::from_str::<serde_json::Value>(&body_str) {
                    return with_request_id(
                        (StatusCode::OK, Json(json_val)).into_response(),
                        &request_id,
                    );
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

fn record_ann_search_metrics(state: &AppState, body: &str) -> bool {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(body) else {
        return false;
    };
    let Some(report) = value.get("ann_report") else {
        return false;
    };
    if report.is_null() {
        return false;
    }
    state.ann_search_requests.fetch_add(1, Ordering::Relaxed);
    if report
        .get("fallback_performed")
        .and_then(|value| value.as_bool())
        .unwrap_or(false)
    {
        state.ann_fallbacks.fetch_add(1, Ordering::Relaxed);
    }
    let Some(decision) = value.get("no_fallback_decision") else {
        return true;
    };
    if decision.is_null() {
        return true;
    }
    state
        .ann_no_fallback_requests
        .fetch_add(1, Ordering::Relaxed);
    if decision
        .get("allowed")
        .and_then(|value| value.as_bool())
        .unwrap_or(false)
    {
        state
            .ann_no_fallback_allowed
            .fetch_add(1, Ordering::Relaxed);
    } else {
        state
            .ann_no_fallback_blocked
            .fetch_add(1, Ordering::Relaxed);
    }
    true
}

fn record_validation_metrics(state: &AppState, body: &str) {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(body) else {
        return;
    };
    if value.get("ok").and_then(|value| value.as_bool()) == Some(false) {
        state.validation_failures.fetch_add(1, Ordering::Relaxed);
    }
}

fn request_id_from_headers(headers: &axum::http::HeaderMap) -> String {
    headers
        .get("x-request-id")
        .and_then(|value| value.to_str().ok())
        .filter(|value| is_safe_request_id(value))
        .map(str::to_owned)
        .unwrap_or_else(next_request_id)
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

fn with_request_id(mut response: Response, request_id: &str) -> Response {
    if let Ok(value) = HeaderValue::from_str(request_id) {
        response.headers_mut().insert("x-request-id", value);
    }
    response
}

fn error_response(code: ErrorCode, message: impl Into<String>) -> ErrorResponse {
    ErrorResponse {
        code,
        error: code.as_str().to_owned(),
        message: message.into(),
    }
}

#[cfg(test)]
mod metrics_tests {
    use super::*;

    fn app_state_for_metrics() -> AppState {
        AppState {
            root: PathBuf::new(),
            dbs: Arc::new(Mutex::new(BTreeMap::new())),
            options: Arc::new(ServerOptions::default()),
            audit_sink: None,
            request_count: Arc::new(AtomicU64::new(0)),
            request_rejected: Arc::new(AtomicU64::new(0)),
            request_duration_ms_total: Arc::new(AtomicU64::new(0)),
            ann_search_requests: Arc::new(AtomicU64::new(0)),
            ann_fallbacks: Arc::new(AtomicU64::new(0)),
            ann_no_fallback_requests: Arc::new(AtomicU64::new(0)),
            ann_no_fallback_allowed: Arc::new(AtomicU64::new(0)),
            ann_no_fallback_blocked: Arc::new(AtomicU64::new(0)),
            ann_search_latency_ms: metrics::LatencyHistogram::new(),
            validation_failures: Arc::new(AtomicU64::new(0)),
            rate_limit: None,
            principal_rate_limits: PrincipalRateLimits::default(),
        }
    }

    #[test]
    fn records_no_fallback_rollout_decision_counters() {
        let state = app_state_for_metrics();
        assert!(record_ann_search_metrics(
            &state,
            r#"{"ann_report":{"fallback_performed":false},"no_fallback_decision":{"allowed":true,"reasons":[]}}"#,
        ));
        assert!(record_ann_search_metrics(
            &state,
            r#"{"ann_report":{"fallback_performed":true},"no_fallback_decision":{"allowed":false,"reasons":["recall_below_minimum"]}}"#,
        ));

        assert_eq!(state.ann_search_requests.load(Ordering::Relaxed), 2);
        assert_eq!(state.ann_fallbacks.load(Ordering::Relaxed), 1);
        assert_eq!(state.ann_no_fallback_requests.load(Ordering::Relaxed), 2);
        assert_eq!(state.ann_no_fallback_allowed.load(Ordering::Relaxed), 1);
        assert_eq!(state.ann_no_fallback_blocked.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn records_ann_search_latency_histogram_buckets() {
        let state = app_state_for_metrics();
        state.ann_search_latency_ms.observe_ms(9);
        state.ann_search_latency_ms.observe_ms(75);
        state.ann_search_latency_ms.observe_ms(1500);

        let buckets = state.ann_search_latency_ms.snapshot();
        assert_eq!(buckets.count, 3);
        assert_eq!(buckets.sum_ms, 1584);
        assert_eq!(buckets.le_10_ms, 1);
        assert_eq!(buckets.le_50_ms, 1);
        assert_eq!(buckets.le_100_ms, 2);
        assert_eq!(buckets.le_500_ms, 2);
        assert_eq!(buckets.le_1000_ms, 2);
        assert_eq!(buckets.gt_1000_ms, 1);
    }
}
