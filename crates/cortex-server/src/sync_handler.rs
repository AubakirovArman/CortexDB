use std::path::Path;

use cortex_engine::Database;

use crate::{dashboard, json_error, json_response, route_shared, ErrorCode, ServerOptions};

fn serve_dashboard() -> String {
    let html = dashboard::html();
    format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\n\r\n{}",
        html.len(),
        html
    )
}

/// ⚠️ TEST-ONLY / COMPATIBILITY-ONLY HARNESS
///
/// This is a **legacy synchronous test harness**, not the production async server path.
/// It opens `Database` directly on every request and wraps it in `RwLock`.
///
/// For production use, always call `serve` or `serve_with_options`, which runs
/// the actor-isolated Tokio/Axum server with per-tenant `DatabaseActor` workers.
///
/// Tests should migrate to the async server path when possible.
pub fn handle_http(root: &Path, request: &str) -> String {
    handle_http_with_options(root, request, &ServerOptions::default())
}

/// ⚠️ TEST-ONLY / COMPATIBILITY-ONLY HARNESS
///
/// See `handle_http` for details. This variant accepts `ServerOptions` for auth-token
/// configuration in legacy integration tests.
pub fn handle_http_with_options(root: &Path, request: &str, options: &ServerOptions) -> String {
    let Some((head, body)) = request.split_once("\r\n\r\n") else {
        return json_error(400, ErrorCode::BadRequest, "bad request");
    };
    let Some(first_line) = head.lines().next() else {
        return json_error(400, ErrorCode::BadRequest, "bad request");
    };
    let parts = first_line.split_whitespace().collect::<Vec<_>>();
    if parts.len() != 3 {
        return json_error(400, ErrorCode::BadRequest, "bad request");
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
            return json_error(
                401,
                ErrorCode::Unauthorized,
                "missing or invalid authorization",
            );
        }
    }

    if parts[1] == "/dashboard" {
        return serve_dashboard();
    }

    let Ok(db) = Database::open(root) else {
        return json_error(500, ErrorCode::Internal, "failed to open database");
    };
    let db = std::sync::RwLock::new(db);
    match route_shared(&db, parts[0], parts[1], body.as_bytes()) {
        Ok(value) => json_response(200, &value),
        Err(error) => {
            let status = match error.as_str() {
                "cell not found" | "job not found" => 404,
                _ => 400,
            };
            json_error(status, ErrorCode::BadRequest, &error)
        }
    }
}
