use serde::Serialize;

#[derive(Serialize, Debug, Clone)]
pub struct HealthResponse {
    pub status: String,
    /// API version path segment (e.g. "v1").
    pub version: String,
    /// Server package version from Cargo.toml.
    pub server_version: String,
}

#[derive(Serialize, Debug, Clone)]
pub struct ClusterNodeResponse {
    pub id: u64,
    pub address: String,
}

#[derive(Serialize, Debug, Clone)]
pub struct ClusterStatusResponse {
    pub local_node: u64,
    pub nodes: Vec<ClusterNodeResponse>,
    pub replication_factor: usize,
    pub distributed_enabled: bool,
}

/// Response metrics containing detailed storage, MemTable, and WAL statistics.
#[derive(Serialize, Debug, Clone)]
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
    pub memtable_payload_bytes: usize,
    /// Estimated in-memory bytes used by MemTable structures and payloads.
    pub estimated_memtable_bytes: usize,
    /// Estimated in-memory bytes used by query/index structures.
    pub estimated_index_bytes: usize,
    /// Estimated bytes needed to materialize a ContextPack working set.
    pub estimated_context_pack_bytes: usize,
    /// Estimated total engine memory across tracked categories.
    pub estimated_total_memory_bytes: usize,
    /// Durable bytes held by live segment bundles.
    pub live_segment_bytes: u64,
    /// Durable bytes held by retired segment bundles waiting for GC.
    pub retired_segment_bytes: u64,
    /// Durable bytes held by all segment bundles.
    pub total_segment_bytes: u64,
    /// Durable segment bytes plus active WAL bytes.
    pub durable_storage_bytes: u64,
    /// Payload bytes inside live segment cells.
    pub live_segment_payload_bytes: u64,
    /// Logical payload proxy used as the denominator for amplification metrics.
    pub logical_payload_bytes: u64,
    /// Q16 durable-storage/logical-payload space amplification proxy.
    pub space_amplification_q16: u32,
    /// Q16 local durable-write/logical-payload amplification proxy.
    pub write_amplification_q16: u32,
    /// Q16 retired-segment/total-segment compaction pressure.
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
}

/// Validation report containing integrity verification results.
#[derive(Serialize, Debug, Clone)]
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

#[derive(Serialize, Debug, Clone)]
pub struct CellResponse {
    pub cell_id: u64,
    pub payload: String,
}

#[derive(Serialize, Debug, Clone)]
pub struct CellLookupResponse {
    pub cell: Option<CellResponse>,
}

#[derive(Serialize, Debug, Clone)]
pub struct PutCellResponse {
    pub seq: u64,
    pub cell_id: u64,
}

#[derive(Serialize, Debug, Clone)]
pub struct DeleteJobResponse {
    pub deleted: bool,
}

#[derive(Serialize, Debug, Clone)]
pub struct CheckpointResponse {
    pub checkpoint_seq: u64,
    pub cells_flushed: usize,
}

#[derive(Serialize, Debug, Clone)]
pub struct CompactionResponse {
    pub compacted: bool,
    pub segments_before: usize,
    pub segments_after: usize,
    pub cells_compacted: usize,
    pub input_bytes: u64,
    pub output_bytes: u64,
    pub duration_ms: u64,
}

#[derive(Serialize, Debug, Clone)]
pub struct CompactionMetricsResponse {
    pub compactions_triggered: u64,
    pub compactions_completed: u64,
    pub compaction_duration_ms_total: u64,
    pub compaction_cells_compacted: u64,
    pub compaction_input_bytes: u64,
}

#[derive(Serialize, Debug, Clone)]
pub struct CompactorStatusResponse {
    pub live_segments: usize,
    pub retired_segments: usize,
    pub compaction: CompactionMetricsResponse,
}

#[derive(Serialize, Debug, Clone)]
pub struct CompactorControlResponse {
    pub background_enabled: bool,
    pub paused: bool,
    pub interval_seconds: u64,
}
