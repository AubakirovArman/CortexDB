use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct HealthResponse {
    pub status: String,
    /// API version path segment (e.g. "v1").
    pub version: String,
    /// Server package version from Cargo.toml.
    pub server_version: String,
}

/// Response metrics containing detailed storage, MemTable, and WAL statistics.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct StatsResponse {
    /// The current global database commit sequence.
    pub current_seq: u64,
    /// The commit sequence of the last successful checkpoint.
    pub checkpoint_seq: u64,
    /// The number of active LSM-segments currently being queried.
    pub live_segments: usize,
    /// The number of garbage-collected or retired segments on disk.
    pub retired_segments: usize,
    /// Total number of unique knowledge cell IDs in MemTable.
    pub memtable_cells: usize,
    /// Total number of cell versions currently held in MemTable.
    pub memtable_versions: usize,
    /// Raw payload bytes currently retained by MemTable versions.
    #[serde(default)]
    pub memtable_payload_bytes: usize,
    /// Estimated in-memory bytes used by MemTable structures and payloads.
    #[serde(default)]
    pub estimated_memtable_bytes: usize,
    /// Estimated in-memory bytes used by query/index structures.
    #[serde(default)]
    pub estimated_index_bytes: usize,
    /// Estimated bytes needed to materialize a ContextPack working set.
    #[serde(default)]
    pub estimated_context_pack_bytes: usize,
    /// Estimated total engine memory across tracked categories.
    #[serde(default)]
    pub estimated_total_memory_bytes: usize,
    /// Durable bytes held by live segment bundles.
    #[serde(default)]
    pub live_segment_bytes: u64,
    /// Durable bytes held by retired segment bundles waiting for GC.
    #[serde(default)]
    pub retired_segment_bytes: u64,
    /// Durable bytes held by all segment bundles.
    #[serde(default)]
    pub total_segment_bytes: u64,
    /// Durable segment bytes plus active WAL bytes.
    #[serde(default)]
    pub durable_storage_bytes: u64,
    /// Payload bytes inside live segment cells.
    #[serde(default)]
    pub live_segment_payload_bytes: u64,
    /// Logical payload proxy used as the denominator for amplification metrics.
    #[serde(default)]
    pub logical_payload_bytes: u64,
    /// Q16 durable-storage/logical-payload space amplification proxy.
    #[serde(default)]
    pub space_amplification_q16: u32,
    /// Q16 local durable-write/logical-payload amplification proxy.
    #[serde(default)]
    pub write_amplification_q16: u32,
    /// Q16 retired-segment/total-segment compaction pressure.
    #[serde(default)]
    pub compaction_pressure_q16: u32,
    /// Total size of the active Write-Ahead Log (.aclog) files in bytes.
    pub wal_size_bytes: u64,
    /// Total number of transaction log records appended.
    pub wal_writer_records: u64,
    /// Total bytes appended to the active WAL file.
    pub wal_writer_bytes: u64,
    /// Total number of disk fsync flushes executed by the WAL writer.
    pub wal_writer_fsyncs: u64,
    /// Total number of batches committed under group commit.
    pub wal_writer_batches: u64,
    /// AQL query-plan cache size, policy, and hit/miss counters.
    #[serde(default)]
    pub aql_query_cache: AqlQueryCacheStatsResponse,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct AqlQueryCacheStatsResponse {
    /// Current cached plan entries.
    pub entries: usize,
    /// Maximum cached plan entries allowed by the configured FIFO policy.
    pub max_entries: usize,
    /// Total cache hits.
    pub hits: u64,
    /// Total cache misses.
    pub misses: u64,
    /// Total FIFO evictions.
    pub evictions: u64,
    /// Total catalog/sequence invalidations.
    pub catalog_invalidations: u64,
    /// Q16 cache hit rate over hits + misses.
    pub hit_rate_q16: u32,
}

impl AqlQueryCacheStatsResponse {
    pub fn from_counts(
        entries: usize,
        max_entries: usize,
        hits: u64,
        misses: u64,
        evictions: u64,
        catalog_invalidations: u64,
    ) -> Self {
        Self {
            entries,
            max_entries,
            hits,
            misses,
            evictions,
            catalog_invalidations,
            hit_rate_q16: hit_rate_q16(hits, misses),
        }
    }
}

fn hit_rate_q16(hits: u64, misses: u64) -> u32 {
    let total = hits.saturating_add(misses);
    if total == 0 {
        0
    } else {
        ((u128::from(hits) * 65_535) / u128::from(total)) as u32
    }
}

/// Validation report containing integrity verification results.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ValidationResponse {
    /// True if there are absolutely zero structural errors or mismatches.
    pub ok: bool,
    /// True if storage manifest is valid.
    pub manifest_ok: bool,
    /// True if WAL records match manifest transactions.
    pub wal_ok: bool,
    pub live_segments_checked: usize,
    pub bitmap_indexes_checked: usize,
    pub lexical_indexes_checked: usize,
    pub vector_indexes_checked: usize,
    pub hnsw_graphs_checked: usize,
    pub cells_checked: usize,
    pub wal_records_checked: u64,
    pub wal_safe_truncate_offset: u64,
    /// List of detected validation errors or warnings.
    pub errors: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CellResponse {
    pub cell_id: u64,
    pub payload: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CellLookupResponse {
    pub cell: Option<CellResponse>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PutCellResponse {
    pub seq: u64,
    pub cell_id: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct WriteBatchRequest {
    pub operations: Vec<WriteBatchOperationRequest>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum WriteBatchOperationRequest {
    PutCell { cell_id: u64, payload: String },
    PatchCell { cell_id: u64, payload: String },
    TombstoneCell { cell_id: u64 },
}

impl WriteBatchOperationRequest {
    pub fn cell_id(&self) -> u64 {
        match self {
            Self::PutCell { cell_id, .. }
            | Self::PatchCell { cell_id, .. }
            | Self::TombstoneCell { cell_id } => *cell_id,
        }
    }
}

impl WriteBatchRequest {
    pub fn new() -> Self {
        Self {
            operations: Vec::new(),
        }
    }

    pub fn put_cell(mut self, cell_id: u64, payload: impl Into<String>) -> Self {
        self.operations.push(WriteBatchOperationRequest::PutCell {
            cell_id,
            payload: payload.into(),
        });
        self
    }

    pub fn patch_cell(mut self, cell_id: u64, payload: impl Into<String>) -> Self {
        self.operations.push(WriteBatchOperationRequest::PatchCell {
            cell_id,
            payload: payload.into(),
        });
        self
    }

    pub fn tombstone_cell(mut self, cell_id: u64) -> Self {
        self.operations
            .push(WriteBatchOperationRequest::TombstoneCell { cell_id });
        self
    }
}

impl Default for WriteBatchRequest {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct WriteBatchResponse {
    pub seq: u64,
    pub operation_count: usize,
    pub cell_ids: Vec<u64>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct RememberResponse {
    pub seq: u64,
    pub cell_id: u64,
    pub ttl_seconds: Option<u64>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeleteJobResponse {
    pub deleted: bool,
}

#[cfg(test)]
mod tests {
    use super::{StatsResponse, WriteBatchRequest};

    #[test]
    fn write_batch_request_wire_shape_is_stable() {
        let request = WriteBatchRequest::new()
            .put_cell(1, "one")
            .patch_cell(1, "two")
            .tombstone_cell(2);
        let value = serde_json::to_value(&request).expect("batch request encodes");
        assert_eq!(
            value,
            serde_json::json!({
                "operations": [
                    {"op": "put_cell", "cell_id": 1, "payload": "one"},
                    {"op": "patch_cell", "cell_id": 1, "payload": "two"},
                    {"op": "tombstone_cell", "cell_id": 2}
                ]
            })
        );
    }

    #[test]
    fn stats_response_decodes_legacy_sdk_shape_with_zero_defaults() {
        let response: StatsResponse = serde_json::from_value(serde_json::json!({
            "current_seq": 1,
            "checkpoint_seq": 0,
            "live_segments": 1,
            "retired_segments": 0,
            "memtable_cells": 2,
            "memtable_versions": 3,
            "wal_size_bytes": 4,
            "wal_writer_records": 5,
            "wal_writer_bytes": 6,
            "wal_writer_fsyncs": 7,
            "wal_writer_batches": 8
        }))
        .expect("legacy shape remains decodable");
        assert_eq!(response.estimated_total_memory_bytes, 0);
        assert_eq!(response.live_segment_bytes, 0);
    }
}
