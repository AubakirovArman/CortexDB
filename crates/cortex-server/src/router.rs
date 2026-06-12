use cortex_aql::{AgentId, AgentView};
use cortex_core::{CellDescriptor, CellId};
use cortex_engine::ClusterConfig;
use cortex_engine::{
    Database, IngestedCell, IngestionBackpressureRequest, IngestionJobId, IngestionProgressTracker,
};

use crate::aql;
use crate::auth::AuthRouteContext;
use crate::authz;
use crate::context;
use crate::hnsw_profile;
use crate::memory;
use crate::responses::{
    AnnMetricsResponse, CellLookupResponse, CellResponse, CheckpointResponse, ClusterNodeResponse,
    ClusterStatusResponse, DeleteJobResponse, ErrorCode, ErrorResponse, HealthResponse,
    IngestResponse, LatencyHistogramResponse, MetricsResponse, PutCellResponse, RouterError,
    StatsResponse, ValidationResponse,
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
    route_database_with_agent(db, method, target, body, None)
}

pub fn route_database_with_agent(
    db: &mut Database,
    method: &str,
    target: &str,
    body: &[u8],
    auth_agent_id: Option<u64>,
) -> Result<String, RouterError> {
    route_database_with_auth(
        db,
        method,
        target,
        body,
        AuthRouteContext::for_agent(auth_agent_id),
    )
}

pub(crate) fn route_database_with_auth(
    db: &mut Database,
    method: &str,
    target: &str,
    body: &[u8],
    auth_context: AuthRouteContext,
) -> Result<String, RouterError> {
    let (path, query) = target.split_once('?').unwrap_or((target, ""));
    let mut authenticated_view = if is_agent_scoped_route(method, path) {
        authz::load_agent_view(db, auth_context.agent_id.map(AgentId))?
    } else {
        None
    };
    if let Some(limit) = auth_context.context_budget_tokens {
        if let Some(view) = authenticated_view.as_mut() {
            clamp_view_context_budget(view, limit);
        }
    }
    match (method, path) {
        ("GET", "/v1/health") => serde_json::to_string(&HealthResponse {
            status: "ok".to_owned(),
            version: "v1".to_owned(),
            server_version: env!("CARGO_PKG_VERSION").to_owned(),
        })
        .map_err(|e| RouterError::BadRequest(e.to_string())),
        ("GET", "/v1/compatibility") => Ok(serde_json::to_string(
            &cortex_engine::compatibility_summary(),
        )?),
        ("GET", "/v1/cluster/status") => {
            let cluster = ClusterConfig::single_node();
            let replication_factor = cluster.nodes.len();
            let response = ClusterStatusResponse {
                local_node: cluster.local_node.0,
                nodes: cluster
                    .nodes
                    .iter()
                    .map(|node| ClusterNodeResponse {
                        id: node.id.0,
                        address: node.address.clone(),
                    })
                    .collect(),
                replication_factor,
                distributed_enabled: replication_factor > 1,
            };
            Ok(serde_json::to_string(&response)?)
        }
        ("GET", "/v1/stats") => {
            let stats = db.storage_stats()?;
            let response = StatsResponse {
                current_seq: stats.current_seq.0,
                checkpoint_seq: stats.checkpoint_seq.0,
                live_segments: stats.live_segments,
                retired_segments: stats.retired_segments,
                memtable_cells: stats.memtable.cell_count,
                memtable_versions: stats.memtable.version_count,
                memtable_payload_bytes: stats.memtable_payload_bytes,
                estimated_memtable_bytes: stats.estimated_memtable_bytes,
                estimated_index_bytes: stats.estimated_index_bytes,
                estimated_context_pack_bytes: stats.estimated_context_pack_bytes,
                estimated_total_memory_bytes: stats.estimated_total_memory_bytes,
                live_segment_bytes: stats.live_segment_bytes,
                retired_segment_bytes: stats.retired_segment_bytes,
                total_segment_bytes: stats.total_segment_bytes,
                durable_storage_bytes: stats.durable_storage_bytes,
                live_segment_payload_bytes: stats.live_segment_payload_bytes,
                logical_payload_bytes: stats.logical_payload_bytes,
                space_amplification_q16: stats.space_amplification_q16,
                write_amplification_q16: stats.write_amplification_q16,
                compaction_pressure_q16: stats.compaction_pressure_q16,
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
            let cell = db.get_latest_cell_with_descriptor(cell_id);
            if let Some((_, descriptor)) = &cell {
                authz::require_descriptor_read(authenticated_view.as_ref(), descriptor)?;
            }
            let response = CellLookupResponse {
                cell: cell.map(|(payload, _)| CellResponse {
                    cell_id: cell_id.0,
                    payload: String::from_utf8_lossy(&payload).into_owned(),
                }),
            };
            Ok(serde_json::to_string(&response)?)
        }
        ("POST", "/put") | ("POST", "/v1/cell") => {
            let cell_id = cell_id(query).map_err(RouterError::BadRequest)?;
            let descriptor = CellDescriptor::from_payload_lossy(body);
            authz::require_descriptor_write(authenticated_view.as_ref(), &descriptor)?;
            let seq = db.put_cell(cell_id, body.to_vec())?;
            let response = PutCellResponse {
                seq: seq.0,
                cell_id: cell_id.0,
            };
            Ok(serde_json::to_string(&response)?)
        }
        ("POST", "/tombstone") | ("DELETE", "/v1/cell") => {
            let cell_id = cell_id(query).map_err(RouterError::BadRequest)?;
            if let Some((_, descriptor)) = db.get_latest_cell_with_descriptor(cell_id) {
                authz::require_descriptor_write(authenticated_view.as_ref(), &descriptor)?;
            }
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
        ("POST", "/v1/context") => context::handle_context_shared(
            db,
            query,
            body,
            authenticated_view.as_ref(),
            Some(&auth_context),
        ),
        ("POST", "/v1/context/trace") => context::handle_context_trace_shared(
            db,
            query,
            body,
            authenticated_view.as_ref(),
            Some(&auth_context),
        ),
        ("POST", "/v1/aql") => aql::handle_aql_shared(db, query, body, authenticated_view.as_ref()),
        ("POST", "/v1/search") => {
            search::handle_search_shared(db, query, body, authenticated_view.as_ref())
        }
        ("POST", "/v1/search/explain") => {
            search::handle_search_explain_shared(db, query, body, authenticated_view.as_ref())
        }
        ("POST", "/v1/search/ann-evaluate") => {
            search::handle_ann_evaluate_shared(db, query, body, authenticated_view.as_ref())
        }
        ("GET", "/v1/admin/search/hnsw/no-fallback-profile") => hnsw_profile::handle_get(db),
        ("PUT", "/v1/admin/search/hnsw/no-fallback-profile") => hnsw_profile::handle_put(db, body),
        ("DELETE", "/v1/admin/search/hnsw/no-fallback-profile") => hnsw_profile::handle_delete(db),
        ("GET", "/v1/metrics") => {
            let stats = db.storage_stats()?;
            let ann = db.ann_metrics();
            let format =
                query_param_opt_decoded(query, "format").unwrap_or_else(|| "json".to_owned());
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
                     # HELP cortexdb_memtable_payload_bytes Raw payload bytes currently retained by MemTable versions.\n\
                     # TYPE cortexdb_memtable_payload_bytes gauge\n\
                     cortexdb_memtable_payload_bytes {}\n\
                     # HELP cortexdb_estimated_memtable_bytes Estimated in-memory bytes used by MemTable structures and payloads.\n\
                     # TYPE cortexdb_estimated_memtable_bytes gauge\n\
                     cortexdb_estimated_memtable_bytes {}\n\
                     # HELP cortexdb_estimated_index_bytes Estimated in-memory bytes used by query/index structures.\n\
                     # TYPE cortexdb_estimated_index_bytes gauge\n\
                     cortexdb_estimated_index_bytes {}\n\
                     # HELP cortexdb_estimated_context_pack_bytes Estimated bytes needed to materialize a ContextPack working set.\n\
                     # TYPE cortexdb_estimated_context_pack_bytes gauge\n\
                     cortexdb_estimated_context_pack_bytes {}\n\
                     # HELP cortexdb_estimated_total_memory_bytes Estimated total engine memory across tracked categories.\n\
                     # TYPE cortexdb_estimated_total_memory_bytes gauge\n\
                     cortexdb_estimated_total_memory_bytes {}\n\
                     # HELP cortexdb_live_segment_bytes Durable bytes held by live segment bundles.\n\
                     # TYPE cortexdb_live_segment_bytes gauge\n\
                     cortexdb_live_segment_bytes {}\n\
                     # HELP cortexdb_retired_segment_bytes Durable bytes held by retired segment bundles waiting for GC.\n\
                     # TYPE cortexdb_retired_segment_bytes gauge\n\
                     cortexdb_retired_segment_bytes {}\n\
                     # HELP cortexdb_total_segment_bytes Durable bytes held by all segment bundles.\n\
                     # TYPE cortexdb_total_segment_bytes gauge\n\
                     cortexdb_total_segment_bytes {}\n\
                     # HELP cortexdb_durable_storage_bytes Durable segment bytes plus active WAL bytes.\n\
                     # TYPE cortexdb_durable_storage_bytes gauge\n\
                     cortexdb_durable_storage_bytes {}\n\
                     # HELP cortexdb_live_segment_payload_bytes Payload bytes inside live segment cells.\n\
                     # TYPE cortexdb_live_segment_payload_bytes gauge\n\
                     cortexdb_live_segment_payload_bytes {}\n\
                     # HELP cortexdb_logical_payload_bytes Logical payload proxy used for amplification ratios.\n\
                     # TYPE cortexdb_logical_payload_bytes gauge\n\
                     cortexdb_logical_payload_bytes {}\n\
                     # HELP cortexdb_space_amplification_q16 Q16 durable-storage/logical-payload space amplification proxy.\n\
                     # TYPE cortexdb_space_amplification_q16 gauge\n\
                     cortexdb_space_amplification_q16 {}\n\
                     # HELP cortexdb_write_amplification_q16 Q16 local durable-write/logical-payload amplification proxy.\n\
                     # TYPE cortexdb_write_amplification_q16 gauge\n\
                     cortexdb_write_amplification_q16 {}\n\
                     # HELP cortexdb_compaction_pressure_q16 Q16 retired-segment/total-segment compaction pressure.\n\
                     # TYPE cortexdb_compaction_pressure_q16 gauge\n\
                     cortexdb_compaction_pressure_q16 {}\n\
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
                    stats.memtable_payload_bytes,
                    stats.estimated_memtable_bytes,
                    stats.estimated_index_bytes,
                    stats.estimated_context_pack_bytes,
                    stats.estimated_total_memory_bytes,
                    stats.live_segment_bytes,
                    stats.retired_segment_bytes,
                    stats.total_segment_bytes,
                    stats.durable_storage_bytes,
                    stats.live_segment_payload_bytes,
                    stats.logical_payload_bytes,
                    stats.space_amplification_q16,
                    stats.write_amplification_q16,
                    stats.compaction_pressure_q16,
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
                    memtable_payload_bytes: stats.memtable_payload_bytes,
                    estimated_memtable_bytes: stats.estimated_memtable_bytes,
                    estimated_index_bytes: stats.estimated_index_bytes,
                    estimated_context_pack_bytes: stats.estimated_context_pack_bytes,
                    estimated_total_memory_bytes: stats.estimated_total_memory_bytes,
                    live_segment_bytes: stats.live_segment_bytes,
                    retired_segment_bytes: stats.retired_segment_bytes,
                    total_segment_bytes: stats.total_segment_bytes,
                    durable_storage_bytes: stats.durable_storage_bytes,
                    live_segment_payload_bytes: stats.live_segment_payload_bytes,
                    logical_payload_bytes: stats.logical_payload_bytes,
                    space_amplification_q16: stats.space_amplification_q16,
                    write_amplification_q16: stats.write_amplification_q16,
                    compaction_pressure_q16: stats.compaction_pressure_q16,
                    wal_size_bytes: stats.wal_size_bytes,
                    wal_writer_records: stats.wal_writer.records_written,
                    wal_writer_bytes: stats.wal_writer.bytes_written,
                    wal_writer_fsyncs: stats.wal_writer.fsync_count,
                    wal_writer_batches: stats.wal_writer.batches_committed,
                    backup_latest_age_seconds: -1,
                    ann_graph_nodes: ann.graph_nodes,
                    ann_total_edges: ann.total_edges,
                    ann_persisted_segments: ann.persisted_segments,
                    ann_has_checkpoint: ann.has_checkpoint,
                    ann_has_uncheckpointed_changes: ann.has_uncheckpointed_changes,
                    ann_search_requests: 0,
                    ann_fallbacks: 0,
                    ann_no_fallback_requests: 0,
                    ann_no_fallback_allowed: 0,
                    ann_no_fallback_blocked: 0,
                    ann_search_latency_ms: LatencyHistogramResponse::default(),
                    actor_queue_depth: 0,
                    actor_queue_capacity: 0,
                    request_count: 0,
                    request_rejected: 0,
                    request_duration_ms_total: 0,
                    request_id_client_provided: 0,
                    request_id_generated: 0,
                    validation_failures: 0,
                    principal_quota_requests_allowed: 0,
                    principal_quota_requests_rejected: 0,
                    principal_quota_body_bytes_allowed: 0,
                    principal_quota_body_bytes_rejected: 0,
                    principal_quota_queue_acquired: 0,
                    principal_quota_queue_rejected: 0,
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
                deleted_vectors: metrics.deleted_vectors,
                rebuild_count: metrics.rebuild_count,
            };
            Ok(serde_json::to_string(&response)?)
        }
        ("POST", "/v1/remember") => {
            memory::handle_remember_shared(db, query, body, authenticated_view.as_ref())
        }
        ("POST", "/v1/forget") => {
            let cell_id = cell_id(query)?;
            if let Some((_, descriptor)) = db.get_latest_cell_with_descriptor(cell_id) {
                authz::require_descriptor_write(authenticated_view.as_ref(), &descriptor)?;
            }
            db.forget_cell(cell_id)?;
            Ok(serde_json::to_string(&PutCellResponse {
                seq: 0,
                cell_id: cell_id.0,
            })?)
        }
        ("POST", "/v1/verify") => {
            memory::handle_verify_shared(db, query, body, authenticated_view.as_ref())
        }
        ("POST", "/v1/ingest/text") => {
            let scope =
                query_param_opt_decoded(query, "scope").unwrap_or_else(|| "default".to_owned());
            authz::require_write_scope_for_optional_view(authenticated_view.as_ref(), &scope)?;
            let source =
                query_param_opt_decoded(query, "source").unwrap_or_else(|| "http_post".to_owned());
            let text = String::from_utf8_lossy(body);
            let start_id = db.allocate_cell_id_range(0);
            let (job_id, results) = track_ingest(db, "ingest_text", None, body.len(), |db| {
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
                validation_report: ingestion_validation_report(db, &results, "ingest_text"),
            };
            Ok(serde_json::to_string(&response)?)
        }
        ("POST", "/v1/ingest/json") => {
            let scope =
                query_param_opt_decoded(query, "scope").unwrap_or_else(|| "default".to_owned());
            authz::require_write_scope_for_optional_view(authenticated_view.as_ref(), &scope)?;
            let source =
                query_param_opt_decoded(query, "source").unwrap_or_else(|| "http_post".to_owned());
            let json = String::from_utf8_lossy(body);
            let start_id = db.allocate_cell_id_range(0);
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
            let scope =
                query_param_opt_decoded(query, "scope").unwrap_or_else(|| "default".to_owned());
            authz::require_write_scope_for_optional_view(authenticated_view.as_ref(), &scope)?;
            let source =
                query_param_opt_decoded(query, "source").unwrap_or_else(|| "http_post".to_owned());
            let csv = String::from_utf8_lossy(body);
            let total = csv.lines().count().saturating_sub(1) as u64;
            let start_id = db.allocate_cell_id_range(0);
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
            let prefix = "/v1/ingest/jobs/";
            let suffix = "/retry";
            let id_str = &path[prefix.len()..path.len() - suffix.len()];
            let id = id_str
                .parse::<u64>()
                .map_err(|_| RouterError::BadRequest("invalid job id".to_owned()))?;
            let progress = db.retry_ingestion_job(id)?;
            Ok(serde_json::to_string(&progress)?)
        }
        _ => Err(RouterError::NotFound("route not found".to_owned())),
    }
}

fn is_agent_scoped_route(method: &str, path: &str) -> bool {
    matches!(
        (method, path),
        ("GET", "/get")
            | ("GET", "/v1/cell")
            | ("POST", "/put")
            | ("POST", "/v1/cell")
            | ("POST", "/tombstone")
            | ("DELETE", "/v1/cell")
            | ("POST", "/v1/context")
            | ("POST", "/v1/context/trace")
            | ("POST", "/v1/aql")
            | ("POST", "/v1/search")
            | ("POST", "/v1/search/explain")
            | ("POST", "/v1/search/ann-evaluate")
            | ("POST", "/v1/remember")
            | ("POST", "/v1/forget")
            | ("POST", "/v1/verify")
            | ("POST", "/v1/ingest/text")
            | ("POST", "/v1/ingest/json")
            | ("POST", "/v1/ingest/csv")
    )
}

/// Legacy/test compatibility wrapper that acquires a write lock and delegates to `route_database`.
/// Prefer `route_database` in production paths where the caller already owns exclusive access.
pub fn route_shared(
    db: &std::sync::RwLock<Database>,
    method: &str,
    target: &str,
    body: &[u8],
) -> Result<String, RouterError> {
    route_shared_with_agent(db, method, target, body, None)
}

pub fn route_shared_with_agent(
    db: &std::sync::RwLock<Database>,
    method: &str,
    target: &str,
    body: &[u8],
    auth_agent_id: Option<u64>,
) -> Result<String, RouterError> {
    let mut db = db
        .write()
        .map_err(|e| RouterError::Internal(e.to_string()))?;
    route_database_with_agent(&mut db, method, target, body, auth_agent_id)
}

#[cfg(test)]
pub(crate) fn route_shared_with_auth(
    db: &std::sync::RwLock<Database>,
    method: &str,
    target: &str,
    body: &[u8],
    auth_context: AuthRouteContext,
) -> Result<String, RouterError> {
    let mut db = db
        .write()
        .map_err(|e| RouterError::Internal(e.to_string()))?;
    route_database_with_auth(&mut db, method, target, body, auth_context)
}

fn clamp_view_context_budget(view: &mut AgentView, limit: u32) {
    view.max_context_budget_tokens = view.max_context_budget_tokens.min(limit);
    view.default_context_budget_tokens = view
        .default_context_budget_tokens
        .min(view.max_context_budget_tokens);
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

pub fn json_error(status: u16, code: ErrorCode, message: &str) -> String {
    let body = serde_json::to_string(&ErrorResponse {
        code,
        error: code.as_str().to_owned(),
        message: message.to_owned(),
    })
    .unwrap_or_else(|_| {
        r#"{"code":"internal","error":"internal","message":"serialization failed"}"#.to_owned()
    });
    json_response(status, &body)
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

pub fn reason(status: u16) -> &'static str {
    match status {
        200 => "OK",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        413 => "Payload Too Large",
        503 => "Service Unavailable",
        500 => "Internal Error",
        _ => "Bad Request",
    }
}
