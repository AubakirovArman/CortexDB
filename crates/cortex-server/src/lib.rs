use axum::{
    extract::{Request, State},
    http::StatusCode,
    response::IntoResponse,
    Json, Router,
};
use std::path::Path;
use tower_http::limit::RequestBodyLimitLayer;

use cortex_core::CellId;
use cortex_engine::Database;

mod aql;
mod context;
mod memory;
mod search;
#[cfg(test)]
mod tests;

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

fn route_shared(
    db: &std::sync::RwLock<Database>,
    method: &str,
    target: &str,
    body: &[u8],
) -> Result<String, String> {
    let (path, query) = target.split_once('?').unwrap_or((target, ""));
    match (method, path) {
        ("GET", "/v1/health") => Ok(r#"{"status":"ok","version":"v1"}"#.to_owned()),
        ("GET", "/v1/stats") => {
            let db = db.read().map_err(|e| e.to_string())?;
            let stats = db.storage_stats().map_err(|error| error.to_string())?;
            Ok(format!(
                r#"{{"current_seq":{},"checkpoint_seq":{},"live_segments":{},"retired_segments":{},"memtable_cells":{},"memtable_versions":{},"wal_size_bytes":{},"wal_writer_records":{},"wal_writer_bytes":{},"wal_writer_fsyncs":{},"wal_writer_batches":{}}}"#,
                stats.current_seq.0,
                stats.checkpoint_seq.0,
                stats.live_segments,
                stats.retired_segments,
                stats.memtable.cell_count,
                stats.memtable.version_count,
                stats.wal_size_bytes,
                stats.wal_writer.records_written,
                stats.wal_writer.bytes_written,
                stats.wal_writer.fsync_count,
                stats.wal_writer.batches_committed
            ))
        }
        ("GET", "/v1/validate") => {
            let db = db.read().map_err(|e| e.to_string())?;
            let validation = db.validate_storage_report();
            Ok(format!(
                r#"{{"ok":{},"manifest_ok":{},"wal_ok":{},"live_segments_checked":{},"bitmap_indexes_checked":{},"lexical_indexes_checked":{},"vector_indexes_checked":{},"hnsw_graphs_checked":{},"cells_checked":{},"wal_records_checked":{},"wal_safe_truncate_offset":{},"errors":[{}]}}"#,
                validation.errors.is_empty(),
                validation.manifest_ok,
                validation.wal_ok,
                validation.live_segments_checked,
                validation.bitmap_indexes_checked,
                validation.lexical_indexes_checked,
                validation.vector_indexes_checked,
                validation.hnsw_graphs_checked,
                validation.cells_checked,
                validation.wal_records_checked,
                validation.wal_safe_truncate_offset,
                json_string_list(&validation.errors)
            ))
        }
        ("GET", "/get") | ("GET", "/v1/cell") => {
            let cell_id = cell_id(query)?;
            let db = db.read().map_err(|e| e.to_string())?;
            let value = db.get_latest_cell(cell_id).map(|payload| {
                format!(
                    r#"{{"cell_id":{},"payload":"{}"}}"#,
                    cell_id.0,
                    escape_json(&String::from_utf8_lossy(&payload))
                )
            });
            Ok(value.unwrap_or_else(|| r#"{"cell":null}"#.to_owned()))
        }
        ("POST", "/put") | ("POST", "/v1/cell") => {
            let cell_id = cell_id(query)?;
            let mut db = db.write().map_err(|e| e.to_string())?;
            let seq = db
                .put_cell(cell_id, body.to_vec())
                .map_err(|error| error.to_string())?;
            Ok(format!(r#"{{"seq":{},"cell_id":{}}}"#, seq.0, cell_id.0))
        }
        ("POST", "/tombstone") | ("DELETE", "/v1/cell") => {
            let cell_id = cell_id(query)?;
            let mut db = db.write().map_err(|e| e.to_string())?;
            let seq = db
                .tombstone_cell(cell_id)
                .map_err(|error| error.to_string())?;
            Ok(format!(r#"{{"seq":{},"cell_id":{}}}"#, seq.0, cell_id.0))
        }
        ("POST", "/flush") | ("POST", "/v1/flush") => {
            let mut db = db.write().map_err(|e| e.to_string())?;
            let stats = db.checkpoint().map_err(|error| error.to_string())?;
            Ok(format!(
                r#"{{"checkpoint_seq":{},"cells_flushed":{}}}"#,
                stats.checkpoint_seq.0, stats.cells_flushed
            ))
        }
        ("POST", "/v1/compact") => {
            let mut db = db.write().map_err(|e| e.to_string())?;
            let stats = db.compact().map_err(|error| error.to_string())?;
            Ok(format!(
                r#"{{"checkpoint_seq":{},"cells_flushed":{}}}"#,
                stats.checkpoint_seq.0, stats.cells_flushed
            ))
        }
        ("POST", "/v1/context") => context::handle_context_shared(db, query, body),
        ("POST", "/v1/aql") => aql::handle_aql_shared(db, query, body),
        ("POST", "/v1/search") => search::handle_search_shared(db, query, body),
        ("POST", "/v1/remember") => memory::handle_remember_shared(db, query, body),
        ("POST", "/v1/verify") => memory::handle_verify_shared(db, query, body),
        ("POST", "/v1/ingest/text") => {
            let scope = query_param_opt(query, "scope").unwrap_or("default");
            let source = query_param_opt(query, "source").unwrap_or("http_post");
            let mut db = db.write().map_err(|e| e.to_string())?;
            let text = String::from_utf8_lossy(body);
            let start_id = db.allocate_cell_id_range(0);
            let results = db
                .ingest_text_chunks(
                    start_id,
                    &text,
                    cortex_engine::TextIngestOptions {
                        scope: scope.to_owned(),
                        source: source.to_owned(),
                    },
                )
                .map_err(|error| error.to_string())?;
            Ok(format!(
                r#"{{"chunks_ingested":{},"first_cell_id":{}}}"#,
                results.len(),
                results[0].cell_id.0
            ))
        }
        ("POST", "/v1/ingest/json") => {
            let scope = query_param_opt(query, "scope").unwrap_or("default");
            let source = query_param_opt(query, "source").unwrap_or("http_post");
            let mut db = db.write().map_err(|e| e.to_string())?;
            let json = String::from_utf8_lossy(body);
            let start_id = db.allocate_cell_id_range(0);
            let results = db
                .ingest_json(
                    start_id,
                    &json,
                    cortex_engine::JsonIngestOptions {
                        scope: scope.to_owned(),
                        source: source.to_owned(),
                    },
                )
                .map_err(|error| error.to_string())?;
            Ok(format!(
                r#"{{"facts_ingested":{},"first_cell_id":{}}}"#,
                results.len(),
                results[0].cell_id.0
            ))
        }
        ("POST", "/v1/ingest/csv") => {
            let scope = query_param_opt(query, "scope").unwrap_or("default");
            let source = query_param_opt(query, "source").unwrap_or("http_post");
            let mut db = db.write().map_err(|e| e.to_string())?;
            let csv = String::from_utf8_lossy(body);
            let start_id = db.allocate_cell_id_range(0);
            let results = db
                .ingest_csv(
                    start_id,
                    &csv,
                    cortex_engine::CsvIngestOptions {
                        scope: scope.to_owned(),
                        source: source.to_owned(),
                    },
                )
                .map_err(|error| error.to_string())?;
            Ok(format!(
                r#"{{"rows_ingested":{},"first_cell_id":{}}}"#,
                results.len(),
                results[0].cell_id.0
            ))
        }
        _ if method == "GET" && path.starts_with("/v1/ingest/jobs/") => {
            let id_str = path.strip_prefix("/v1/ingest/jobs/").unwrap();
            let id = id_str
                .parse::<u64>()
                .map_err(|_| "invalid job id".to_owned())?;
            let db = db.read().map_err(|e| e.to_string())?;
            let progress = db
                .load_ingestion_job(id)
                .map_err(|error| error.to_string())?;
            if let Some(p) = progress {
                let content = serde_json::to_string(&p).map_err(|e| e.to_string())?;
                Ok(content)
            } else {
                Err("job not found".to_owned())
            }
        }
        _ => Err("unknown route".to_owned()),
    }
}

fn query_param_opt<'a>(query: &'a str, key: &str) -> Option<&'a str> {
    let prefix = format!("{key}=");
    query.split('&').find_map(|pair| pair.strip_prefix(&prefix))
}

fn cell_id(query: &str) -> Result<CellId, String> {
    query_param(query, "cell_id")?
        .parse::<u64>()
        .map(CellId)
        .map_err(|_| "cell_id must be u64".to_owned())
}

fn query_param<'a>(query: &'a str, key: &str) -> Result<&'a str, String> {
    let prefix = format!("{key}=");
    query
        .split('&')
        .find_map(|pair| pair.strip_prefix(&prefix))
        .ok_or_else(|| format!("missing {key}"))
}

fn json_response(status: u16, body: &str) -> String {
    let reason = reason(status);
    format!(
        "HTTP/1.1 {status} {reason}\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\r\n{body}",
        body.len()
    )
}

fn json_error(status: u16, code: &str, message: &str) -> String {
    json_response(
        status,
        &format!(
            r#"{{"error":"{}","message":"{}"}}"#,
            escape_json(code),
            escape_json(message)
        ),
    )
}

fn reason(status: u16) -> &'static str {
    match status {
        200 => "OK",
        401 => "Unauthorized",
        404 => "Not Found",
        413 => "Payload Too Large",
        500 => "Internal Error",
        _ => "Bad Request",
    }
}

fn escape_json(value: &str) -> String {
    value
        .chars()
        .flat_map(|character| match character {
            '"' => "\\\"".chars().collect::<Vec<_>>(),
            '\\' => "\\\\".chars().collect(),
            '\n' => "\\n".chars().collect(),
            '\r' => "\\r".chars().collect(),
            '\t' => "\\t".chars().collect(),
            other => vec![other],
        })
        .collect()
}

fn json_string_list(values: &[String]) -> String {
    values
        .iter()
        .map(|value| format!(r#""{}""#, escape_json(value)))
        .collect::<Vec<_>>()
        .join(",")
}
