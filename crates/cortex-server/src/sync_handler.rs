use std::path::Path;

use cortex_engine::Database;

use crate::{
    auth, auth_policy_store, dashboard, json_error, json_response, llm, route_shared_with_agent,
    ErrorCode, ServerOptions,
};

fn serve_dashboard() -> String {
    let html = dashboard::html();
    format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\n\r\n{}",
        html.len(),
        html
    )
}

fn serve_dashboard_asset(path: &str) -> Option<String> {
    let asset = dashboard::asset(path)?;
    Some(format!(
        "HTTP/1.1 200 OK\r\nContent-Type: {}\r\nContent-Length: {}\r\n\r\n{}",
        asset.content_type,
        asset.body.len(),
        asset.body
    ))
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

    let path = parts[1].split_once('?').map_or(parts[1], |(path, _)| path);
    let auth_header = head.lines().skip(1).find_map(|line| {
        line.strip_prefix("authorization:")
            .or_else(|| line.strip_prefix("Authorization:"))
            .map(|val| val.trim())
    });
    let auth_decision = match auth::authorize_request(options, auth_header, parts[0], path) {
        Ok(decision) => decision,
        Err(error) => return json_error(error.status_code(), error.code(), &error.to_string()),
    };

    if dashboard::is_page(parts[1]) {
        return serve_dashboard();
    }
    if let Some(response) = serve_dashboard_asset(parts[1]) {
        return response;
    }

    let query = parts[1].split_once('?').map_or("", |(_, query)| query);
    match auth_policy_store::handle_admin_request(options, parts[0], path, query, body.as_bytes()) {
        Ok(Some(value)) => return json_response(200, &value),
        Ok(None) => {}
        Err(error) => return json_error(error.status_code(), error.code(), &error.to_string()),
    }

    if parts[0] == "POST" && path == "/v1/inference" {
        return match llm::handle_inference_test_double(
            body.as_bytes(),
            options.llm_test_double_enabled,
        ) {
            Ok(value) => json_response(200, &value),
            Err(error) => json_error(error.status_code(), error.code(), &error.to_string()),
        };
    }

    let Ok(db) = Database::open(root) else {
        return json_error(500, ErrorCode::Internal, "failed to open database");
    };
    let db = std::sync::RwLock::new(db);
    match route_shared_with_agent(
        &db,
        parts[0],
        parts[1],
        body.as_bytes(),
        auth_decision.agent_id,
    ) {
        Ok(value) => json_response(200, &value),
        Err(error) => json_error(error.status_code(), error.code(), &error.to_string()),
    }
}
