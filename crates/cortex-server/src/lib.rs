use axum::{
    extract::{Request, State},
    http::StatusCode,
    response::IntoResponse,
    routing::Router,
    Json,
};
use std::path::Path;
use tower_http::limit::RequestBodyLimitLayer;

use cortex_engine::Database;

mod aql;
mod context;
mod memory;
mod router;
mod search;
pub mod responses;
#[cfg(test)]
mod tests;

pub use router::{
    cell_id, escape_json, json_error, json_response, json_string_list, query_param,
    query_param_opt, route_shared,
};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ServerOptions {
    pub auth_token: Option<String>,
}

#[derive(Clone)]
pub struct AppState {
    db: std::sync::Arc<std::sync::RwLock<Database>>,
    options: std::sync::Arc<ServerOptions>,
}

pub fn serve(root: &Path, addr: &str) -> std::io::Result<()> {
    serve_with_options(root, addr, ServerOptions::default())
}

pub fn serve_with_options(root: &Path, addr: &str, options: ServerOptions) -> std::io::Result<()> {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    rt.block_on(async {
        let db = Database::open(root).map_err(|error| std::io::Error::other(error.to_string()))?;
        let state = AppState {
            db: std::sync::Arc::new(std::sync::RwLock::new(db)),
            options: std::sync::Arc::new(options),
        };

        let app = Router::new()
            .fallback(axum_handler)
            .layer(RequestBodyLimitLayer::new(2 * 1024 * 1024)) // 2MB Limit
            .with_state(state);

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
                Json(serde_json::json!({
                    "error": "unauthorized",
                    "message": "missing or invalid authorization"
                })),
            )
                .into_response();
        }
    }

    let body_bytes = match axum::body::to_bytes(req.into_body(), 2 * 1024 * 1024).await {
        Ok(bytes) => bytes.to_vec(),
        Err(_) => {
            return (
                StatusCode::PAYLOAD_TOO_LARGE,
                Json(serde_json::json!({
                    "error": "payload_too_large",
                    "message": "request body exceeds 2MB limit"
                })),
            )
                .into_response();
        }
    };

    let target = if query.is_empty() {
        path
    } else {
        format!("{path}?{query}")
    };
    match route_shared(&state.db, &method, &target, &body_bytes) {
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
            (
                status,
                Json(serde_json::json!({
                    "error": "bad_request",
                    "message": err_msg
                })),
            )
                .into_response()
        }
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
