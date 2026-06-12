use crate::responses::{
    AnnMetricsResponse, LatencyHistogramResponse, MetricsResponse, RouterError,
};

use super::params::query_param_opt_decoded;
use super::DatabaseAccess;

pub(super) fn try_route<A: DatabaseAccess>(
    db: &mut A,
    method: &str,
    path: &str,
    query: &str,
) -> Option<Result<String, RouterError>> {
    if !matches!(
        (method, path),
        ("GET", "/v1/metrics") | ("GET", "/v1/ann/metrics")
    ) {
        return None;
    }
    Some((|| -> Result<String, RouterError> {
        match (method, path) {
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
                        active_readers: 0,
                        waiting_writers: 0,
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
                        compactions_triggered: 0,
                        compactions_completed: 0,
                        compaction_duration_ms_total: 0,
                        compaction_cells_compacted: 0,
                        compaction_input_bytes: 0,
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
            _ => unreachable!("route prefiltered"),
        }
    })())
}
