use cortex_core::CellId;
use cortex_engine::{Database, IngestionJobId, IngestionProgressTracker};

use crate::aql;
use crate::context;
use crate::memory;
use crate::responses::{
    AnnMetricsResponse, CellLookupResponse, CellResponse, CheckpointResponse, ErrorResponse,
    HealthResponse, IngestResponse, MetricsResponse, PutCellResponse, RouterError, StatsResponse,
    ValidationResponse,
};
use crate::search;

/// Production route entrypoint used by `DatabaseActor`.
/// Operates directly on `&mut Database` because the actor guarantees
/// single-threaded sequential access, eliminating the need for an inner `RwLock`.
pub fn route_database(
    db: &mut Database,
    method: &str,
    target: &str,
    body: &[u8],
) -> Result<String, RouterError> {
    let (path, query) = target.split_once('?').unwrap_or((target, ""));
    match (method, path) {
        ("GET", "/v1/health") => serde_json::to_string(&HealthResponse {
            status: "ok".to_owned(),
            version: "v1".to_owned(),
        })
        .map_err(|e| RouterError::BadRequest(e.to_string())),
        ("GET", "/v1/stats") => {
            let stats = db.storage_stats()?;
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
            Ok(serde_json::to_string(&response)?)
        }
        ("GET", "/v1/validate") => {
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
            Ok(serde_json::to_string(&response)?)
        }
        ("GET", "/get") | ("GET", "/v1/cell") => {
            let cell_id = cell_id(query).map_err(RouterError::BadRequest)?;
            let response = CellLookupResponse {
                cell: db.get_latest_cell(cell_id).map(|payload| CellResponse {
                    cell_id: cell_id.0,
                    payload: String::from_utf8_lossy(&payload).into_owned(),
                }),
            };
            Ok(serde_json::to_string(&response)?)
        }
        ("POST", "/put") | ("POST", "/v1/cell") => {
            let cell_id = cell_id(query).map_err(RouterError::BadRequest)?;
            let seq = db.put_cell(cell_id, body.to_vec())?;
            let response = PutCellResponse {
                seq: seq.0,
                cell_id: cell_id.0,
            };
            Ok(serde_json::to_string(&response)?)
        }
        ("POST", "/tombstone") | ("DELETE", "/v1/cell") => {
            let cell_id = cell_id(query).map_err(RouterError::BadRequest)?;
            let seq = db.tombstone_cell(cell_id)?;
            let response = PutCellResponse {
                seq: seq.0,
                cell_id: cell_id.0,
            };
            Ok(serde_json::to_string(&response)?)
        }
        ("POST", "/flush") | ("POST", "/v1/flush") => {
            let stats = db.checkpoint()?;
            let response = CheckpointResponse {
                checkpoint_seq: stats.checkpoint_seq.0,
                cells_flushed: stats.cells_flushed,
            };
            Ok(serde_json::to_string(&response)?)
        }
        ("POST", "/v1/compact") => {
            let stats = db.compact()?;
            let response = CheckpointResponse {
                checkpoint_seq: stats.checkpoint_seq.0,
                cells_flushed: stats.cells_flushed,
            };
            Ok(serde_json::to_string(&response)?)
        }
        ("POST", "/v1/context") => {
            context::handle_context_shared(db, query, body).map_err(RouterError::BadRequest)
        }
        ("POST", "/v1/aql") => {
            aql::handle_aql_shared(db, query, body).map_err(RouterError::BadRequest)
        }
        ("POST", "/v1/search") => {
            search::handle_search_shared(db, query, body).map_err(RouterError::BadRequest)
        }
        ("POST", "/v1/search/explain") => {
            search::handle_search_explain_shared(db, query, body).map_err(RouterError::BadRequest)
        }
        ("POST", "/v1/search/ann-evaluate") => {
            search::handle_ann_evaluate_shared(db, query, body).map_err(RouterError::BadRequest)
        }
        ("GET", "/v1/metrics") => {
            let stats = db.storage_stats()?;
            let ann = db.ann_metrics();
            let format = query_param_opt(query, "format").unwrap_or("json");
            if format == "prometheus" {
                let lines = format!(
                    "# HELP cortexdb_current_seq Current database commit sequence.\n\
                     # TYPE cortexdb_current_seq gauge\n\
                     cortexdb_current_seq {}\n\
                     # HELP cortexdb_checkpoint_seq Last checkpoint commit sequence.\n\
                     # TYPE cortexdb_checkpoint_seq gauge\n\
                     cortexdb_checkpoint_seq {}\n\
                     # HELP cortexdb_live_segments Number of active LSM segments.\n\
                     # TYPE cortexdb_live_segments gauge\n\
                     cortexdb_live_segments {}\n\
                     # HELP cortexdb_retired_segments Number of retired LSM segments.\n\
                     # TYPE cortexdb_retired_segments gauge\n\
                     cortexdb_retired_segments {}\n\
                     # HELP cortexdb_memtable_cells Number of unique cells in memtable.\n\
                     # TYPE cortexdb_memtable_cells gauge\n\
                     cortexdb_memtable_cells {}\n\
                     # HELP cortexdb_memtable_versions Number of cell versions in memtable.\n\
                     # TYPE cortexdb_memtable_versions gauge\n\
                     cortexdb_memtable_versions {}\n\
                     # HELP cortexdb_wal_size_bytes Total WAL size in bytes.\n\
                     # TYPE cortexdb_wal_size_bytes gauge\n\
                     cortexdb_wal_size_bytes {}\n\
                     # HELP cortexdb_wal_writer_records Total WAL records written.\n\
                     # TYPE cortexdb_wal_writer_records counter\n\
                     cortexdb_wal_writer_records {}\n\
                     # HELP cortexdb_wal_writer_bytes Total WAL bytes written.\n\
                     # TYPE cortexdb_wal_writer_bytes counter\n\
                     cortexdb_wal_writer_bytes {}\n\
                     # HELP cortexdb_wal_writer_fsyncs Total WAL fsync calls.\n\
                     # TYPE cortexdb_wal_writer_fsyncs counter\n\
                     cortexdb_wal_writer_fsyncs {}\n\
                     # HELP cortexdb_wal_writer_batches Total WAL batches committed.\n\
                     # TYPE cortexdb_wal_writer_batches counter\n\
                     cortexdb_wal_writer_batches {}\n\
                     # HELP cortexdb_ann_graph_nodes Number of ANN graph nodes.\n\
                     # TYPE cortexdb_ann_graph_nodes gauge\n\
                     cortexdb_ann_graph_nodes {}\n\
                     # HELP cortexdb_ann_total_edges Total ANN graph edges.\n\
                     # TYPE cortexdb_ann_total_edges gauge\n\
                     cortexdb_ann_total_edges {}\n\
                     # HELP cortexdb_ann_persisted_segments Number of persisted ANN segments.\n\
                     # TYPE cortexdb_ann_persisted_segments gauge\n\
                     cortexdb_ann_persisted_segments {}\n",
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
                    stats.wal_writer.batches_committed,
                    ann.graph_nodes,
                    ann.total_edges,
                    ann.persisted_segments,
                );
                Ok(lines)
            } else {
                let response = MetricsResponse {
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
                    ann_graph_nodes: ann.graph_nodes,
                    ann_total_edges: ann.total_edges,
                    ann_persisted_segments: ann.persisted_segments,
                    ann_has_checkpoint: ann.has_checkpoint,
                    ann_has_uncheckpointed_changes: ann.has_uncheckpointed_changes,
                    actor_queue_depth: 0,
                    actor_queue_capacity: 0,
                    request_count: 0,
                    request_duration_ms_total: 0,
                };
                Ok(serde_json::to_string(&response)?)
            }
        }
        ("GET", "/v1/ann/metrics") => {
            let metrics = db.ann_metrics();
            let response = AnnMetricsResponse {
                graph_nodes: metrics.graph_nodes,
                total_edges: metrics.total_edges,
                persisted_segments: metrics.persisted_segments,
                has_checkpoint: metrics.has_checkpoint,
                has_uncheckpointed_changes: metrics.has_uncheckpointed_changes,
            };
            Ok(serde_json::to_string(&response)?)
        }
        ("POST", "/v1/remember") => {
            memory::handle_remember_shared(db, query, body).map_err(RouterError::BadRequest)
        }
        ("POST", "/v1/verify") => {
            memory::handle_verify_shared(db, query, body).map_err(RouterError::BadRequest)
        }
        ("POST", "/v1/ingest/text") => {
            let scope = query_param_opt(query, "scope").unwrap_or("default");
            let source = query_param_opt(query, "source").unwrap_or("http_post");
            let text = String::from_utf8_lossy(body);
            let start_id = db.allocate_cell_id_range(0);
            let (job_id, results) = track_ingest(db, "ingest_text", None, |db| {
                db.ingest_text_chunks(
                    start_id,
                    &text,
                    cortex_engine::TextIngestOptions {
                        scope: scope.to_owned(),
                        source: source.to_owned(),
                    },
                )
            })?;
            let response = IngestResponse {
                rows_ingested: 0,
                chunks_ingested: results.len(),
                facts_ingested: 0,
                first_cell_id: results.first().map(|cell| cell.cell_id.0),
                job_id: Some(job_id.0),
            };
            Ok(serde_json::to_string(&response)?)
        }
        ("POST", "/v1/ingest/json") => {
            let scope = query_param_opt(query, "scope").unwrap_or("default");
            let source = query_param_opt(query, "source").unwrap_or("http_post");
            let json = String::from_utf8_lossy(body);
            let start_id = db.allocate_cell_id_range(0);
            let (job_id, results) = track_ingest(db, "ingest_json", None, |db| {
                db.ingest_json(
                    start_id,
                    &json,
                    cortex_engine::JsonIngestOptions {
                        scope: scope.to_owned(),
                        source: source.to_owned(),
                    },
                )
            })?;
            let response = IngestResponse {
                rows_ingested: 0,
                chunks_ingested: 0,
                facts_ingested: results.len(),
                first_cell_id: results.first().map(|cell| cell.cell_id.0),
                job_id: Some(job_id.0),
            };
            Ok(serde_json::to_string(&response)?)
        }
        ("POST", "/v1/ingest/csv") => {
            let scope = query_param_opt(query, "scope").unwrap_or("default");
            let source = query_param_opt(query, "source").unwrap_or("http_post");
            let csv = String::from_utf8_lossy(body);
            let total = csv.lines().count().saturating_sub(1) as u64;
            let start_id = db.allocate_cell_id_range(0);
            let (job_id, results) = track_ingest(db, "ingest_csv", Some(total), |db| {
                db.ingest_csv(
                    start_id,
                    &csv,
                    cortex_engine::CsvIngestOptions {
                        scope: scope.to_owned(),
                        source: source.to_owned(),
                    },
                )
            })?;
            let response = IngestResponse {
                rows_ingested: results.len(),
                chunks_ingested: 0,
                facts_ingested: 0,
                first_cell_id: results.first().map(|cell| cell.cell_id.0),
                job_id: Some(job_id.0),
            };
            Ok(serde_json::to_string(&response)?)
        }
        ("GET", "/v1/ingest/jobs") => {
            let jobs = db.list_ingestion_jobs()?;
            Ok(serde_json::to_string(&jobs)?)
        }
        _ if method == "GET" && path.starts_with("/v1/ingest/jobs/") => {
            let id_str = path.strip_prefix("/v1/ingest/jobs/").unwrap();
            let id = id_str
                .parse::<u64>()
                .map_err(|_| RouterError::BadRequest("invalid job id".to_owned()))?;
            let progress = db.load_ingestion_job(id)?;
            if let Some(p) = progress {
                let content = serde_json::to_string(&p).map_err(|e| e.to_string())?;
                Ok(content)
            } else {
                Err(RouterError::NotFound("job not found".to_owned()))
            }
        }
        _ => Err(RouterError::BadRequest("unknown route".to_owned())),
    }
}

/// Legacy/test compatibility wrapper that acquires a write lock and delegates to `route_database`.
/// Prefer `route_database` in production paths where the caller already owns exclusive access.
pub fn route_shared(
    db: &std::sync::RwLock<Database>,
    method: &str,
    target: &str,
    body: &[u8],
) -> Result<String, String> {
    let mut db = db.write().map_err(|e| e.to_string())?;
    route_database(&mut db, method, target, body).map_err(|e| e.to_string())
}

pub fn query_param_opt<'a>(query: &'a str, key: &str) -> Option<&'a str> {
    let prefix = format!("{key}=");
    query.split('&').find_map(|pair| pair.strip_prefix(&prefix))
}

/// Returns the percent-decoded value of a query parameter.
/// Falls back to the raw value if decoding fails.
pub fn query_param_decoded(query: &str, key: &str) -> Result<String, String> {
    let raw = query_param(query, key)?;
    decode_percent(raw)
}

/// Returns the percent-decoded value of an optional query parameter.
/// Falls back to the raw value if decoding fails.
pub fn query_param_opt_decoded(query: &str, key: &str) -> Option<String> {
    query_param_opt(query, key).and_then(|raw| decode_percent(raw).ok())
}

fn decode_percent(raw: &str) -> Result<String, String> {
    // Replace '+' with space for application/x-www-form-urlencoded compatibility
    let normalized = raw.replace('+', " ");
    percent_encoding::percent_decode_str(&normalized)
        .decode_utf8()
        .map(|cow| cow.into_owned())
        .map_err(|e| format!("invalid percent-encoding: {e}"))
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
    let body = serde_json::to_string(&ErrorResponse {
        error: code.to_owned(),
        message: message.to_owned(),
    })
    .unwrap_or_else(|_| {
        r#"{"error":"internal_error","message":"serialization failed"}"#.to_owned()
    });
    json_response(status, &body)
}

fn track_ingest<T>(
    db: &mut Database,
    label: &str,
    total_items: Option<u64>,
    ingest: impl FnOnce(&mut Database) -> Result<T, cortex_engine::error::EngineError>,
) -> Result<(IngestionJobId, T), RouterError> {
    let mut tracker = IngestionProgressTracker::default();
    tracker.seed_next_id_from_disk(db);
    let job_id = tracker
        .start(label, total_items)
        .map_err(|e| RouterError::Internal(e.to_string()))?;
    let _ = db.save_ingestion_job(tracker.get(job_id).unwrap());
    match ingest(db) {
        Ok(result) => {
            tracker
                .finish(job_id)
                .map_err(|e| RouterError::Internal(e.to_string()))?;
            let _ = db.save_ingestion_job(tracker.get(job_id).unwrap());
            Ok((job_id, result))
        }
        Err(error) => {
            let _ = tracker.fail(job_id, error.to_string());
            let _ = db.save_ingestion_job(tracker.get(job_id).unwrap());
            Err(error.into())
        }
    }
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
