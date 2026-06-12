use cortex_core::CellDescriptor;
use cortex_engine::ClusterConfig;

use crate::authz;
use crate::responses::{
    CellLookupResponse, CellResponse, CheckpointResponse, ClusterNodeResponse,
    ClusterStatusResponse, CompactionMetricsResponse, CompactionResponse, CompactorStatusResponse,
    HealthResponse, PutCellResponse, RouterError, StatsResponse, ValidationResponse,
};

use super::params::cell_id;
use super::DatabaseAccess;

pub(super) fn try_route<A: DatabaseAccess>(
    db: &mut A,
    method: &str,
    path: &str,
    query: &str,
    body: &[u8],
    authenticated_view: Option<&cortex_aql::AgentView>,
) -> Option<Result<String, RouterError>> {
    if !matches!(
        (method, path),
        ("GET", "/v1/health")
            | ("GET", "/v1/compatibility")
            | ("GET", "/v1/cluster/status")
            | ("GET", "/v1/stats")
            | ("GET", "/v1/validate")
            | ("GET", "/get")
            | ("GET", "/v1/cell")
            | ("POST", "/put")
            | ("POST", "/v1/cell")
            | ("POST", "/tombstone")
            | ("DELETE", "/v1/cell")
            | ("POST", "/flush")
            | ("POST", "/v1/flush")
            | ("POST", "/v1/compact")
            | ("POST", "/v1/admin/compact/trigger")
            | ("GET", "/v1/admin/compact/status")
    ) {
        return None;
    }
    Some((|| -> Result<String, RouterError> {
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
                    authz::require_descriptor_read(authenticated_view, descriptor)?;
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
                let db = db
                    .as_write()
                    .ok_or_else(|| RouterError::Internal("write route on read lock".to_owned()))?;
                let cell_id = cell_id(query).map_err(RouterError::BadRequest)?;
                let descriptor = CellDescriptor::from_payload_lossy(body);
                authz::require_descriptor_write(authenticated_view, &descriptor)?;
                let seq = db.put_cell(cell_id, body.to_vec())?;
                let response = PutCellResponse {
                    seq: seq.0,
                    cell_id: cell_id.0,
                };
                Ok(serde_json::to_string(&response)?)
            }
            ("POST", "/tombstone") | ("DELETE", "/v1/cell") => {
                let db = db
                    .as_write()
                    .ok_or_else(|| RouterError::Internal("write route on read lock".to_owned()))?;
                let cell_id = cell_id(query).map_err(RouterError::BadRequest)?;
                if let Some((_, descriptor)) = db.get_latest_cell_with_descriptor(cell_id) {
                    authz::require_descriptor_write(authenticated_view, &descriptor)?;
                }
                let seq = db.tombstone_cell(cell_id)?;
                let response = PutCellResponse {
                    seq: seq.0,
                    cell_id: cell_id.0,
                };
                Ok(serde_json::to_string(&response)?)
            }
            ("POST", "/flush") | ("POST", "/v1/flush") => {
                let db = db
                    .as_write()
                    .ok_or_else(|| RouterError::Internal("write route on read lock".to_owned()))?;
                let stats = db.checkpoint()?;
                let response = CheckpointResponse {
                    checkpoint_seq: stats.checkpoint_seq.0,
                    cells_flushed: stats.cells_flushed,
                };
                Ok(serde_json::to_string(&response)?)
            }
            ("POST", "/v1/compact") => {
                let db = db
                    .as_write()
                    .ok_or_else(|| RouterError::Internal("write route on read lock".to_owned()))?;
                let stats = db.compact()?;
                let response = CheckpointResponse {
                    checkpoint_seq: stats.checkpoint_seq.0,
                    cells_flushed: stats.cells_flushed,
                };
                Ok(serde_json::to_string(&response)?)
            }
            ("POST", "/v1/admin/compact/trigger") => {
                let db = db
                    .as_write()
                    .ok_or_else(|| RouterError::Internal("write route on read lock".to_owned()))?;
                let stats = db.incremental_compact()?;
                let response = CompactionResponse {
                    compacted: stats.segments_before > stats.segments_after,
                    segments_before: stats.segments_before,
                    segments_after: stats.segments_after,
                    cells_compacted: stats.cells_compacted,
                    input_bytes: stats.input_bytes,
                    output_bytes: stats.output_bytes,
                    duration_ms: stats.duration_ms,
                };
                Ok(serde_json::to_string(&response)?)
            }
            ("GET", "/v1/admin/compact/status") => {
                let stats = db.storage_stats()?;
                let response = CompactorStatusResponse {
                    live_segments: stats.live_segments,
                    retired_segments: stats.retired_segments,
                    compaction: CompactionMetricsResponse {
                        compactions_triggered: stats.compactions_triggered,
                        compactions_completed: stats.compactions_completed,
                        compaction_duration_ms_total: stats.compaction_duration_ms_total,
                        compaction_cells_compacted: stats.compaction_cells_compacted,
                        compaction_input_bytes: stats.compaction_input_bytes,
                    },
                };
                Ok(serde_json::to_string(&response)?)
            }
            _ => unreachable!("route prefiltered"),
        }
    })())
}
