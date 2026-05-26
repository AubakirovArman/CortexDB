use cortex_core::CellId;
use cortex_engine::Database;

use crate::aql;
use crate::context;
use crate::memory;
use crate::responses::{
    CellResponse, IngestResponse, PutCellResponse, StatsResponse, ValidationResponse,
};
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
            let response = StatsResponse {
                current_seq: stats.current_seq.0,
                checkpoint_seq: stats.checkpoint_seq.0,
                live_segments: stats.live_segments,
                retired_segments: stats.retired_segments,
                memtable_cells: stats.memtable.cell_count,
                memtable_versions: stats.memtable.version_count,
                wal_size_bytes: stats.wal_size_bytes,
                wal_writer_records: stats.wal_writer.records_written,
                wal_writer_bytes: stats.wal_writer.bytes_written,
                wal_writer_fsyncs: stats.wal_writer.fsync_count,
                wal_writer_batches: stats.wal_writer.batches_committed,
            };
            serde_json::to_string(&response).map_err(|e| e.to_string())
        }
        ("GET", "/v1/validate") => {
            let db = db.read().map_err(|e| e.to_string())?;
            let validation = db.validate_storage_report();
            let response = ValidationResponse {
                ok: validation.errors.is_empty(),
                manifest_ok: validation.manifest_ok,
                wal_ok: validation.wal_ok,
                live_segments_checked: validation.live_segments_checked,
                bitmap_indexes_checked: validation.bitmap_indexes_checked,
                lexical_indexes_checked: validation.lexical_indexes_checked,
                vector_indexes_checked: validation.vector_indexes_checked,
                hnsw_graphs_checked: validation.hnsw_graphs_checked,
                cells_checked: validation.cells_checked,
                wal_records_checked: validation.wal_records_checked as u64,
                wal_safe_truncate_offset: validation.wal_safe_truncate_offset,
                errors: validation.errors,
            };
            serde_json::to_string(&response).map_err(|e| e.to_string())
        }
        ("GET", "/get") | ("GET", "/v1/cell") => {
            let cell_id = cell_id(query)?;
            let db = db.read().map_err(|e| e.to_string())?;
            let value = db.get_latest_cell(cell_id).map(|payload| CellResponse {
                cell_id: cell_id.0,
                payload: String::from_utf8_lossy(&payload).into_owned(),
            });
            if let Some(res) = value {
                serde_json::to_string(&res).map_err(|e| e.to_string())
            } else {
                Ok(r#"{"cell":null}"#.to_owned())
            }
        }
        ("POST", "/put") | ("POST", "/v1/cell") => {
            let cell_id = cell_id(query)?;
            let mut db = db.write().map_err(|e| e.to_string())?;
            let seq = db
                .put_cell(cell_id, body.to_vec())
                .map_err(|error| error.to_string())?;
            let response = PutCellResponse {
                seq: seq.0,
                cell_id: cell_id.0,
            };
            serde_json::to_string(&response).map_err(|e| e.to_string())
        }
        ("POST", "/tombstone") | ("DELETE", "/v1/cell") => {
            let cell_id = cell_id(query)?;
            let mut db = db.write().map_err(|e| e.to_string())?;
            let seq = db
                .tombstone_cell(cell_id)
                .map_err(|error| error.to_string())?;
            let response = PutCellResponse {
                seq: seq.0,
                cell_id: cell_id.0,
            };
            serde_json::to_string(&response).map_err(|e| e.to_string())
        }
        ("POST", "/flush") | ("POST", "/v1/flush") => {
            let mut db = db.write().map_err(|e| e.to_string())?;
            let stats = db.checkpoint().map_err(|error| error.to_string())?;
            let response = serde_json::json!({
                "checkpoint_seq": stats.checkpoint_seq.0,
                "cells_flushed": stats.cells_flushed
            });
            serde_json::to_string(&response).map_err(|e| e.to_string())
        }
        ("POST", "/v1/compact") => {
            let mut db = db.write().map_err(|e| e.to_string())?;
            let stats = db.compact().map_err(|error| error.to_string())?;
            let response = serde_json::json!({
                "checkpoint_seq": stats.checkpoint_seq.0,
                "cells_flushed": stats.cells_flushed
            });
            serde_json::to_string(&response).map_err(|e| e.to_string())
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
            let response = IngestResponse {
                rows_ingested: 0,
                chunks_ingested: results.len(),
                facts_ingested: 0,
                first_cell_id: results.first().map(|cell| cell.cell_id.0),
            };
            serde_json::to_string(&response).map_err(|e| e.to_string())
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
            let response = IngestResponse {
                rows_ingested: 0,
                chunks_ingested: 0,
                facts_ingested: results.len(),
                first_cell_id: results.first().map(|cell| cell.cell_id.0),
            };
            serde_json::to_string(&response).map_err(|e| e.to_string())
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
            let response = IngestResponse {
                rows_ingested: results.len(),
                chunks_ingested: 0,
                facts_ingested: 0,
                first_cell_id: results.first().map(|cell| cell.cell_id.0),
            };
            serde_json::to_string(&response).map_err(|e| e.to_string())
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
