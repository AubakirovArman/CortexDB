use serde::Serialize;

pub use cortex_api_types::core::{
    AqlQueryCacheStatsResponse, CellLookupResponse, CellResponse, DeleteJobResponse,
    HealthResponse, PutCellResponse, StatsResponse, ValidationResponse, WriteBatchOperationRequest,
    WriteBatchRequest, WriteBatchResponse,
};

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
