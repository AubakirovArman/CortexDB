use cortex_aql::AgentView;
use cortex_engine::{
    Database, IngestedCell, IngestionBackpressureRequest, IngestionJobId, IngestionProgressTracker,
};

use crate::authz;
use crate::embedding;
use crate::responses::{DeleteJobResponse, IngestResponse, RouterError};

use super::params::query_param_opt_decoded;
use super::DatabaseAccess;

pub(super) fn try_route<A: DatabaseAccess>(
    db: &mut A,
    method: &str,
    path: &str,
    query: &str,
    body: &[u8],
    authenticated_view: Option<&AgentView>,
) -> Option<Result<String, RouterError>> {
    let is_ingest_job_route = path.starts_with("/v1/ingest/jobs/");
    if !matches!(
        (method, path),
        ("POST", "/v1/ingest/text")
            | ("POST", "/v1/ingest/json")
            | ("POST", "/v1/ingest/csv")
            | ("GET", "/v1/ingest/jobs")
    ) && !is_ingest_job_route
    {
        return None;
    }
    Some((|| -> Result<String, RouterError> {
        match (method, path) {
            ("POST", "/v1/ingest/text") => {
                let db = db
                    .as_write()
                    .ok_or_else(|| RouterError::Internal("write route on read lock".to_owned()))?;
                let scope =
                    query_param_opt_decoded(query, "scope").unwrap_or_else(|| "default".to_owned());
                authz::require_write_scope_for_optional_view(authenticated_view, &scope)?;
                let source = query_param_opt_decoded(query, "source")
                    .unwrap_or_else(|| "http_post".to_owned());
                let text = String::from_utf8_lossy(body);
                let expected_cells = cortex_engine::count_text_chunks(
                    &source,
                    &text,
                    cortex_engine::TextChunkPolicy::default(),
                )?;
                let embed =
                    embedding::parse_bool_param(query_param_opt_decoded(query, "embed"), "embed")?;
                let embedder = if embed {
                    Some(
                        embedding::embedder_from_env()?
                            .ok_or_else(embedding::missing_vector_or_config_error)?,
                    )
                } else {
                    None
                };
                let start_id = db.allocate_cell_id_range(expected_cells)?;
                let (job_id, results) = track_ingest(db, "ingest_text", None, body.len(), |db| {
                    let options = cortex_engine::TextIngestOptions {
                        scope: scope.to_owned(),
                        source: source.to_owned(),
                    };
                    match &embedder {
                        Some(embedder) => db.ingest_text_chunks_with_embedder(
                            start_id,
                            &text,
                            options,
                            cortex_engine::TextChunkPolicy::default(),
                            cortex_engine::IngestionUpdatePolicy::AlwaysInsert,
                            embedder,
                        ),
                        None => db.ingest_text_chunks(start_id, &text, options),
                    }
                })?;
                let response = IngestResponse {
                    rows_ingested: 0,
                    chunks_ingested: results.len(),
                    facts_ingested: 0,
                    first_cell_id: results.first().map(|cell| cell.cell_id.0),
                    job_id: Some(job_id.0),
                    validation_report: ingestion_validation_report(db, &results, "ingest_text"),
                };
                Ok(serde_json::to_string(&response)?)
            }
            ("POST", "/v1/ingest/json") => {
                let db = db
                    .as_write()
                    .ok_or_else(|| RouterError::Internal("write route on read lock".to_owned()))?;
                let scope =
                    query_param_opt_decoded(query, "scope").unwrap_or_else(|| "default".to_owned());
                authz::require_write_scope_for_optional_view(authenticated_view, &scope)?;
                let source = query_param_opt_decoded(query, "source")
                    .unwrap_or_else(|| "http_post".to_owned());
                let json = String::from_utf8_lossy(body);
                let expected_cells = cortex_engine::count_json_ingest_cells(&json)?;
                let start_id = db.allocate_cell_id_range(expected_cells)?;
                let (job_id, results) = track_ingest(db, "ingest_json", None, body.len(), |db| {
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
                    validation_report: ingestion_validation_report(db, &results, "ingest_json"),
                };
                Ok(serde_json::to_string(&response)?)
            }
            ("POST", "/v1/ingest/csv") => {
                let db = db
                    .as_write()
                    .ok_or_else(|| RouterError::Internal("write route on read lock".to_owned()))?;
                let scope =
                    query_param_opt_decoded(query, "scope").unwrap_or_else(|| "default".to_owned());
                authz::require_write_scope_for_optional_view(authenticated_view, &scope)?;
                let source = query_param_opt_decoded(query, "source")
                    .unwrap_or_else(|| "http_post".to_owned());
                let csv = String::from_utf8_lossy(body);
                let total = csv.lines().count().saturating_sub(1) as u64;
                let expected_cells = cortex_engine::count_csv_ingest_cells(&csv)?;
                let start_id = db.allocate_cell_id_range(expected_cells)?;
                let (job_id, results) =
                    track_ingest(db, "ingest_csv", Some(total), body.len(), |db| {
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
                    validation_report: ingestion_validation_report(db, &results, "ingest_csv"),
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
            _ if method == "DELETE" && path.starts_with("/v1/ingest/jobs/") => {
                let db = db
                    .as_write()
                    .ok_or_else(|| RouterError::Internal("write route on read lock".to_owned()))?;
                let id_str = path.strip_prefix("/v1/ingest/jobs/").unwrap();
                let id = id_str
                    .parse::<u64>()
                    .map_err(|_| RouterError::BadRequest("invalid job id".to_owned()))?;
                let deleted = db.delete_ingestion_job(id)?;
                if deleted {
                    let response = DeleteJobResponse { deleted };
                    Ok(serde_json::to_string(&response)?)
                } else {
                    Err(RouterError::NotFound("job not found".to_owned()))
                }
            }
            _ if method == "POST"
                && path.starts_with("/v1/ingest/jobs/")
                && path.ends_with("/cancel") =>
            {
                let db = db
                    .as_write()
                    .ok_or_else(|| RouterError::Internal("write route on read lock".to_owned()))?;
                let prefix = "/v1/ingest/jobs/";
                let suffix = "/cancel";
                let id_str = &path[prefix.len()..path.len() - suffix.len()];
                let id = id_str
                    .parse::<u64>()
                    .map_err(|_| RouterError::BadRequest("invalid job id".to_owned()))?;
                let progress = db.cancel_ingestion_job(id)?;
                Ok(serde_json::to_string(&progress)?)
            }
            _ if method == "POST"
                && path.starts_with("/v1/ingest/jobs/")
                && path.ends_with("/retry") =>
            {
                let db = db
                    .as_write()
                    .ok_or_else(|| RouterError::Internal("write route on read lock".to_owned()))?;
                let prefix = "/v1/ingest/jobs/";
                let suffix = "/retry";
                let id_str = &path[prefix.len()..path.len() - suffix.len()];
                let id = id_str
                    .parse::<u64>()
                    .map_err(|_| RouterError::BadRequest("invalid job id".to_owned()))?;
                let progress = db.retry_ingestion_job(id)?;
                Ok(serde_json::to_string(&progress)?)
            }
            _ => unreachable!("route prefiltered"),
        }
    })())
}

fn track_ingest(
    db: &mut Database,
    label: &str,
    total_items: Option<u64>,
    input_bytes: usize,
    ingest: impl FnOnce(&mut Database) -> Result<Vec<IngestedCell>, cortex_engine::EngineError>,
) -> Result<(IngestionJobId, Vec<IngestedCell>), RouterError> {
    db.check_ingestion_backpressure(IngestionBackpressureRequest {
        input_bytes,
        total_items,
    })?;
    let mut tracker = IngestionProgressTracker::default();
    tracker.seed_next_id_from_disk(db);
    let job_id = tracker
        .start(label, total_items)
        .map_err(|e| RouterError::Internal(e.to_string()))?;
    let progress = tracker
        .get(job_id)
        .ok_or_else(|| RouterError::Internal("ingestion job disappeared".to_owned()))?;
    db.save_ingestion_job(progress)?;
    db.ensure_ingestion_job_not_cancelled(job_id)?;
    match ingest(db) {
        Ok(result) => {
            for cell in &result {
                tracker
                    .record_cell(job_id, cell.cell_id)
                    .map_err(|e| RouterError::Internal(e.to_string()))?;
            }
            tracker
                .finish(job_id)
                .map_err(|e| RouterError::Internal(e.to_string()))?;
            let progress = tracker
                .get(job_id)
                .ok_or_else(|| RouterError::Internal("ingestion job disappeared".to_owned()))?;
            db.save_ingestion_job(progress)?;
            Ok((job_id, result))
        }
        Err(error) => {
            let _ = tracker.fail(job_id, error.to_string());
            if let Some(progress) = tracker.get(job_id) {
                let _ = db.save_ingestion_job(progress);
            }
            Err(error.into())
        }
    }
}

fn ingestion_validation_report(
    db: &Database,
    cells: &[IngestedCell],
    input_ref: &str,
) -> cortex_engine::IngestionValidationReport {
    let mut report = db.ingestion_validation_report(cells);
    if cells.is_empty() {
        report.record_skipped("no_cells_emitted", Some(input_ref.to_owned()));
    }
    report
}
