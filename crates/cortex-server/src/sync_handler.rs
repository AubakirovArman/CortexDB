use std::path::Path;

use cortex_engine::Database;

use crate::receipt::ReceiptEmissionContext;
use crate::responses::ErrorCode;
use crate::router::{query_param_opt_decoded, route_shared_with_auth};
use crate::{
    auth, auth_agent_admin, auth_policy_cells, auth_policy_store, auth_scope_admin, dashboard,
    json_error, json_response, llm, validate_tenant_id, ServerOptions,
};

fn serve_dashboard() -> String {
    let html = dashboard::html();
    let security_headers = dashboard_security_headers();
    format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\n{security_headers}Cache-Control: no-cache\r\nContent-Length: {}\r\n\r\n{}",
        html.len(),
        html
    )
}

fn serve_dashboard_asset(path: &str) -> Option<String> {
    let asset = dashboard::asset(path)?;
    let security_headers = dashboard_security_headers();
    Some(format!(
        "HTTP/1.1 200 OK\r\nContent-Type: {}\r\n{security_headers}Cache-Control: public, max-age=31536000, immutable\r\nContent-Length: {}\r\n\r\n{}",
        asset.content_type,
        asset.body.len(),
        asset.body
    ))
}

fn dashboard_security_headers() -> String {
    dashboard::SECURITY_HEADERS
        .iter()
        .map(|(name, value)| format!("{name}: {value}\r\n"))
        .collect()
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
    let resolved_options;
    let receipt_signer_configured =
        options.receipt_signing_key.is_some() || options.receipt_external_signer.is_some();
    let options = if receipt_signer_configured && options.db_instance_id.is_none() {
        match crate::database_identity::with_database_instance_id(root, options) {
            Ok(value) => {
                resolved_options = value;
                &resolved_options
            }
            Err(error) => return json_error(500, ErrorCode::Internal, &error.to_string()),
        }
    } else {
        options
    };
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

    if options.dashboard_enabled && dashboard::is_page(parts[1]) {
        return serve_dashboard();
    }
    if options.dashboard_enabled {
        if let Some(response) = serve_dashboard_asset(parts[1]) {
            return response;
        }
    }

    let query = parts[1].split_once('?').map_or("", |(_, query)| query);
    match auth_policy_store::handle_admin_request(options, parts[0], path, query, body.as_bytes()) {
        Ok(Some(value)) => {
            if value.sync_policy_cells {
                let sync_result =
                    open_database(root, options)
                        .map_err(|_| ())
                        .and_then(|mut db| {
                            auth_policy_cells::sync_store_json_to_database(
                                &mut db,
                                &value.policy_store_json,
                            )
                            .map(|_| ())
                            .map_err(|_| ())
                        });
                if sync_result.is_err() {
                    return json_error(
                        500,
                        ErrorCode::Internal,
                        "auth policy store persisted but policy cell sync failed",
                    );
                }
            }
            return json_response(200, &value.body);
        }
        Ok(None) => {}
        Err(error) => return json_error(error.status_code(), error.code(), &error.to_string()),
    }

    if path == "/v1/agents" || path.starts_with("/v1/agents/") {
        let Ok(mut db) = open_database(root, options) else {
            return json_error(500, ErrorCode::Internal, "failed to open database");
        };
        return match auth_agent_admin::handle_sync_request(&mut db, parts[0], path, body.as_bytes())
        {
            Ok(Some(body)) => json_response(200, &body),
            Ok(None) => json_error(404, ErrorCode::NotFound, "unknown agent admin route"),
            Err(error) => json_error(error.status_code(), error.code(), &error.to_string()),
        };
    }

    if parts[0] == "POST"
        && matches!(
            path,
            "/v1/admin/auth/scope/grant" | "/v1/admin/auth/scope/revoke"
        )
    {
        let grant = path == "/v1/admin/auth/scope/grant";
        let (agent_id, scope, access) =
            match auth_scope_admin::parse_scope_mutation_body(body.as_bytes()) {
                Ok(value) => value,
                Err(error) => {
                    return json_error(error.status_code(), error.code(), &error.to_string())
                }
            };
        let Ok(mut db) = open_database(root, options) else {
            return json_error(500, ErrorCode::Internal, "failed to open database");
        };
        return match auth_scope_admin::apply_agent_scope_mutation(
            &mut db, agent_id, &scope, access, grant,
        ) {
            Ok(response) => match serde_json::to_string(&response) {
                Ok(body) => json_response(200, &body),
                Err(error) => json_error(500, ErrorCode::Internal, &error.to_string()),
            },
            Err(error) => json_error(error.status_code(), error.code(), &error.to_string()),
        };
    }

    if parts[0] == "POST" && path == "/v1/inference" {
        return match llm::handle_inference_test_double(
            body.as_bytes(),
            options.llm_test_double_enabled,
        ) {
            Ok(value) => json_response(200, &value.body),
            Err(error) => json_error(
                error.error.status_code(),
                error.error.code(),
                &error.error.to_string(),
            ),
        };
    }
    if parts[0] == "GET" && path == "/v1/cluster/status" {
        return match serde_json::to_string(&crate::cluster::status_response(options)) {
            Ok(body) => json_response(200, &body),
            Err(error) => json_error(500, ErrorCode::Internal, &error.to_string()),
        };
    }
    if let Some(decision) = crate::cluster::context_ingress_decision(options, parts[0], path) {
        match decision {
            crate::cluster::ContextIngressDecision::Local => {}
            crate::cluster::ContextIngressDecision::Forward(target) => {
                return match crate::cluster::forward_http_request(
                    &target,
                    parts[0],
                    parts[1],
                    body.as_bytes(),
                    auth_header,
                    None,
                    None,
                ) {
                    Ok(response) => json_response(response.status_code, &response.body),
                    Err(error) => json_error(503, ErrorCode::ServiceUnavailable, error.as_str()),
                };
            }
            crate::cluster::ContextIngressDecision::Unavailable(message) => {
                return json_error(503, ErrorCode::ServiceUnavailable, &message);
            }
        }
    }

    let tenant = query_param_opt_decoded(query, "tenant").unwrap_or_else(|| "default".to_owned());
    if !validate_tenant_id(&tenant) {
        return json_error(
            400,
            ErrorCode::InvalidTenant,
            "invalid tenant ID structure. Only alphanumeric, '_', and '-' up to 64 characters are allowed.",
        );
    }
    if !auth::tenant_can_access(&auth_decision, &tenant) {
        return json_error(
            403,
            ErrorCode::Forbidden,
            "token tenant policy is not allowed to access this tenant",
        );
    }

    let Ok(db) = open_tenant_database(root, options, &tenant) else {
        return json_error(500, ErrorCode::Internal, "failed to open database");
    };
    let db = std::sync::RwLock::new(db);
    let receipt_context = match ReceiptEmissionContext::from_options(options) {
        Ok(value) => value,
        Err(error) => return json_error(error.status_code(), error.code(), &error.to_string()),
    };
    match route_shared_with_auth(
        &db,
        parts[0],
        parts[1],
        body.as_bytes(),
        auth_decision.route_context(),
        receipt_context.as_ref(),
    ) {
        Ok(value) => json_response(200, &value),
        Err(error) => json_error(error.status_code(), error.code(), &error.to_string()),
    }
}

fn open_database(root: &Path, options: &ServerOptions) -> cortex_engine::EngineResult<Database> {
    Database::open_with_options(root, options.engine_database_options.clone())
}

fn open_tenant_database(
    root: &Path,
    options: &ServerOptions,
    tenant: &str,
) -> cortex_engine::EngineResult<Database> {
    if tenant == "default" {
        return open_database(root, options);
    }
    let tenant_path = root.join("realms").join(tenant);
    std::fs::create_dir_all(&tenant_path)?;
    open_database(&tenant_path, options)
}
