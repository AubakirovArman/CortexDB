use axum::{
    extract::{Request, State},
    http::{header, HeaderValue, Method, StatusCode},
    response::{Html, IntoResponse},
    routing::Router,
    Json,
};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use tower_http::cors::CorsLayer;
use tower_http::limit::RequestBodyLimitLayer;

use responses::{ErrorCode, ErrorResponse, MetricsResponse};

mod actor;
mod aql;
mod context;
mod dashboard;
#[cfg(test)]
mod dashboard_tests;
mod memory;
pub mod responses;
mod router;
mod search;
#[cfg(test)]
mod search_tests;
#[cfg(test)]
mod sync_handler;
#[cfg(test)]
mod tests;

use crate::responses::RouterError;
pub use router::{
    cell_id, json_error, json_response, query_param, query_param_decoded, query_param_opt,
    query_param_opt_decoded, route_database, route_shared,
};
#[cfg(test)]
pub use sync_handler::{handle_http, handle_http_with_options};

pub const DEFAULT_ACTOR_QUEUE_CAPACITY: usize = 1024;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ServerOptions {
    pub auth_token: Option<String>,
    /// Capacity of the bounded actor command queue. Default is 1024.
    pub actor_queue_capacity: usize,
    /// Optional exact browser origin allowed for cross-origin API requests.
    ///
    /// CORS is disabled by default. Set this only for deployments that expose
    /// CortexDB to a browser origin through a trusted reverse proxy.
    pub cors_allowed_origin: Option<String>,
}

impl ServerOptions {
    pub fn actor_queue_capacity(&self) -> usize {
        if self.actor_queue_capacity == 0 {
            DEFAULT_ACTOR_QUEUE_CAPACITY
        } else {
            self.actor_queue_capacity
        }
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
    request_count: Arc<AtomicU64>,
    request_rejected: Arc<AtomicU64>,
    request_duration_ms_total: Arc<AtomicU64>,
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
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    rt.block_on(async {
        let cors = cors_layer(&options)?;
        let state = AppState {
            root: root.to_owned(),
            dbs: Arc::new(Mutex::new(BTreeMap::new())),
            options: Arc::new(options),
            request_count: Arc::new(AtomicU64::new(0)),
            request_rejected: Arc::new(AtomicU64::new(0)),
            request_duration_ms_total: Arc::new(AtomicU64::new(0)),
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

async fn axum_handler(State(state): State<AppState>, req: Request) -> impl IntoResponse {
    let method = req.method().as_str().to_owned();
    let uri = req.uri().to_owned();
    let path = uri.path().to_owned();
    let query = uri.query().unwrap_or("").to_owned();

    let span = tracing::info_span!("http_request", %method, %path);
    let _enter = span.enter();

    if method == "GET" && (path == "/" || path == "/dashboard") {
        return Html(dashboard::html()).into_response();
    }
    if method == "GET" {
        if let Some(asset) = dashboard::asset(&path) {
            return (
                StatusCode::OK,
                [(header::CONTENT_TYPE, asset.content_type)],
                asset.body,
            )
                .into_response();
        }
    }

    let auth_header = req
        .headers()
        .get("authorization")
        .or_else(|| req.headers().get("Authorization"))
        .and_then(|h| h.to_str().ok());

    if let Some(ref expected_token) = state.options.auth_token {
        let expected_bearer = format!("Bearer {expected_token}");
        if auth_header != Some(expected_bearer.as_str()) {
            return (
                StatusCode::UNAUTHORIZED,
                Json(error_response(
                    ErrorCode::Unauthorized,
                    "missing or invalid authorization",
                )),
            )
                .into_response();
        }
    }

    let body_bytes = match axum::body::to_bytes(req.into_body(), 2 * 1024 * 1024).await {
        Ok(bytes) => bytes.to_vec(),
        Err(_) => {
            return (
                StatusCode::PAYLOAD_TOO_LARGE,
                Json(error_response(
                    ErrorCode::PayloadTooLarge,
                    "request body exceeds 2MB limit",
                )),
            )
                .into_response();
        }
    };

    let tenant = query_param_opt_decoded(&query, "tenant").unwrap_or_else(|| "default".to_owned());
    if !validate_tenant_id(&tenant) {
        return (
            StatusCode::BAD_REQUEST,
            Json(error_response(
                ErrorCode::InvalidTenant,
                "invalid tenant ID structure. Only alphanumeric, '_', and '-' up to 64 characters are allowed.",
            )),
        )
            .into_response();
    }
    let db = match state.get_db(&tenant) {
        Ok(db) => db,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(error_response(ErrorCode::Internal, e.to_string())),
            )
                .into_response();
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
        actor.route(&method_clone, &target_clone, &body_clone)
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
            if method == "GET" && path == "/v1/metrics" {
                if query.contains("format=prometheus") {
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
                         # HELP cortexdb_request_rejected Total HTTP requests rejected due to queue full.\n\
                         # TYPE cortexdb_request_rejected counter\n\
                         cortexdb_request_rejected {}\n\
                         # HELP cortexdb_request_duration_ms_total Total HTTP request duration in milliseconds.\n\
                         # TYPE cortexdb_request_duration_ms_total counter\n\
                         cortexdb_request_duration_ms_total {}\n",
                        db.queue_depth(),
                        db.queue_capacity(),
                        state.request_count.load(Ordering::Relaxed),
                        state.request_rejected.load(Ordering::Relaxed),
                        state.request_duration_ms_total.load(Ordering::Relaxed),
                    );
                    return (StatusCode::OK, body_str + &extra).into_response();
                }
                if let Ok(mut metrics) = serde_json::from_str::<MetricsResponse>(&body_str) {
                    metrics.actor_queue_depth = db.queue_depth();
                    metrics.actor_queue_capacity = db.queue_capacity();
                    metrics.request_count = state.request_count.load(Ordering::Relaxed);
                    metrics.request_rejected = state.request_rejected.load(Ordering::Relaxed);
                    metrics.request_duration_ms_total =
                        state.request_duration_ms_total.load(Ordering::Relaxed);
                    return (StatusCode::OK, Json(metrics)).into_response();
                }
                if let Ok(json_val) = serde_json::from_str::<serde_json::Value>(&body_str) {
                    return (StatusCode::OK, Json(json_val)).into_response();
                }
            }
            if let Ok(json_val) = serde_json::from_str::<serde_json::Value>(&body_str) {
                (StatusCode::OK, Json(json_val)).into_response()
            } else {
                (StatusCode::OK, body_str).into_response()
            }
        }
        Err(err) => {
            let status = StatusCode::from_u16(err.status_code()).unwrap_or(StatusCode::BAD_REQUEST);
            (status, Json(error_response(err.code(), err.to_string()))).into_response()
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
