use cortex_core::CellId;
use cortex_engine::Database;

use crate::aql;
use crate::context;
use crate::memory;
use crate::search;

pub fn route_shared(
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

pub fn query_param_opt<'a>(query: &'a str, key: &str) -> Option<&'a str> {
    let prefix = format!("{key}=");
    query.split('&').find_map(|pair| pair.strip_prefix(&prefix))
}

pub fn cell_id(query: &str) -> Result<CellId, String> {
    query_param(query, "cell_id")?
        .parse::<u64>()
        .map(CellId)
        .map_err(|_| "cell_id must be u64".to_owned())
}

pub fn query_param<'a>(query: &'a str, key: &str) -> Result<&'a str, String> {
    let prefix = format!("{key}=");
    query
        .split('&')
        .find_map(|pair| pair.strip_prefix(&prefix))
        .ok_or_else(|| format!("missing {key}"))
}

pub fn json_response(status: u16, body: &str) -> String {
    let reason = reason(status);
    format!(
        "HTTP/1.1 {status} {reason}\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\r\n{body}",
        body.len()
    )
}

pub fn json_error(status: u16, code: &str, message: &str) -> String {
    json_response(
        status,
        &format!(
            r#"{{"error":"{}","message":"{}"}}"#,
            escape_json(code),
            escape_json(message)
        ),
    )
}

pub fn reason(status: u16) -> &'static str {
    match status {
        200 => "OK",
        401 => "Unauthorized",
        404 => "Not Found",
        413 => "Payload Too Large",
        500 => "Internal Error",
        _ => "Bad Request",
    }
}

pub fn escape_json(value: &str) -> String {
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

pub fn json_string_list(values: &[String]) -> String {
    values
        .iter()
        .map(|value| format!(r#""{}""#, escape_json(value)))
        .collect::<Vec<_>>()
        .join(",")
}
