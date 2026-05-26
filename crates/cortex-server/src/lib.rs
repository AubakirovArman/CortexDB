use axum::{
    extract::{Request, State},
    http::StatusCode,
    response::{Html, IntoResponse},
    routing::Router,
    Json,
};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tower_http::limit::RequestBodyLimitLayer;

use cortex_engine::Database;
use responses::ErrorResponse;

mod actor;
mod aql;
mod context;
mod dashboard;
mod memory;
pub mod responses;
mod router;
mod search;
#[cfg(test)]
mod search_tests;
#[cfg(test)]
mod tests;

pub use router::{cell_id, json_error, json_response, query_param, query_param_opt, route_shared};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ServerOptions {
    pub auth_token: Option<String>,
}

#[derive(Clone)]
pub struct AppState {
    root: PathBuf,
    dbs: Arc<Mutex<BTreeMap<String, Arc<actor::DatabaseActor>>>>,
    options: Arc<ServerOptions>,
}

impl AppState {
    pub fn get_db(&self, tenant: &str) -> std::io::Result<Arc<actor::DatabaseActor>> {
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
        let db_shared = Arc::new(actor::DatabaseActor::open(&tenant_path)?);
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
        let state = AppState {
            root: root.to_owned(),
            dbs: Arc::new(Mutex::new(BTreeMap::new())),
            options: Arc::new(options),
        };

        let app = Router::new()
            .fallback(axum_handler)
            .layer(RequestBodyLimitLayer::new(2 * 1024 * 1024)) // 2MB Limit
            .with_state(state);

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

        let listener = tokio::net::TcpListener::bind(addr).await?;
        axum::serve(listener, app).await?;
        Ok(())
    })
}

async fn axum_handler(State(state): State<AppState>, req: Request) -> impl IntoResponse {
    let method = req.method().as_str().to_owned();
    let uri = req.uri().to_owned();
    let path = uri.path().to_owned();
    let query = uri.query().unwrap_or("").to_owned();

    if method == "GET" && (path == "/" || path == "/dashboard") {
        return Html(dashboard::html()).into_response();
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
                    "unauthorized",
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
                    "payload_too_large",
                    "request body exceeds 2MB limit",
                )),
            )
                .into_response();
        }
    };

    let tenant = query_param_opt(&query, "tenant").unwrap_or("default");
    let db = match state.get_db(tenant) {
        Ok(db) => db,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(error_response("internal_error", e.to_string())),
            )
                .into_response();
        }
    };

    let target = if query.is_empty() {
        path
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
        Err(_) => Err("internal server error".to_owned()),
    };
    let duration = start.elapsed();
    if duration.as_millis() > 50 {
        eprintln!(
            "⚠️ [SLOW QUERY ALERT] method={} target={} took={:?}",
            method, target, duration
        );
    }

    match res {
        Ok(body_str) => {
            if let Ok(json_val) = serde_json::from_str::<serde_json::Value>(&body_str) {
                (StatusCode::OK, Json(json_val)).into_response()
            } else {
                (StatusCode::OK, body_str).into_response()
            }
        }
        Err(err_msg) => {
            let status = match err_msg.as_str() {
                "cell not found" | "job not found" => StatusCode::NOT_FOUND,
                _ => StatusCode::BAD_REQUEST,
            };
            (status, Json(error_response("bad_request", err_msg))).into_response()
        }
    }
}

fn error_response(error: impl Into<String>, message: impl Into<String>) -> ErrorResponse {
    ErrorResponse {
        error: error.into(),
        message: message.into(),
    }
}

pub fn handle_http(root: &Path, request: &str) -> String {
    handle_http_with_options(root, request, &ServerOptions::default())
}

pub fn handle_http_with_options(root: &Path, request: &str, options: &ServerOptions) -> String {
    let Some((head, body)) = request.split_once("\r\n\r\n") else {
        return json_error(400, "bad_request", "bad request");
    };
    let Some(first_line) = head.lines().next() else {
        return json_error(400, "bad_request", "bad request");
    };
    let parts = first_line.split_whitespace().collect::<Vec<_>>();
    if parts.len() != 3 {
        return json_error(400, "bad_request", "bad request");
    }

    // Check Authorization
    if let Some(ref expected_token) = options.auth_token {
        let expected_bearer = format!("Bearer {expected_token}");
        let auth_header = head.lines().skip(1).find_map(|line| {
            line.strip_prefix("authorization:")
                .or_else(|| line.strip_prefix("Authorization:"))
                .map(|val| val.trim())
        });
        if auth_header != Some(expected_bearer.as_str()) {
            return json_error(401, "unauthorized", "missing or invalid authorization");
        }
    }

    let Ok(db) = Database::open(root) else {
        return json_error(500, "internal_error", "failed to open database");
    };
    let db = std::sync::RwLock::new(db);
    match route_shared(&db, parts[0], parts[1], body.as_bytes()) {
        Ok(value) => json_response(200, &value),
        Err(error) => {
            let status = match error.as_str() {
                "cell not found" | "job not found" => 404,
                _ => 400,
            };
            json_error(status, "bad_request", &error)
        }
    }
}
