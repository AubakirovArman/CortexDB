use serde::Deserialize;

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub server_version: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct StatsResponse {
    pub current_seq: u64,
    pub checkpoint_seq: u64,
    pub live_segments: usize,
    pub retired_segments: usize,
    pub memtable_cells: usize,
    pub memtable_versions: usize,
    pub memtable_payload_bytes: usize,
    pub estimated_memtable_bytes: usize,
    pub estimated_index_bytes: usize,
    pub estimated_context_pack_bytes: usize,
    pub estimated_total_memory_bytes: usize,
    pub wal_size_bytes: u64,
    pub wal_writer_records: u64,
    pub wal_writer_bytes: u64,
    pub wal_writer_fsyncs: u64,
    pub wal_writer_batches: u64,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct ValidationResponse {
    pub ok: bool,
    pub manifest_ok: bool,
    pub wal_ok: bool,
    pub live_segments_checked: usize,
    pub bitmap_indexes_checked: usize,
    pub lexical_indexes_checked: usize,
    pub vector_indexes_checked: usize,
    pub hnsw_graphs_checked: usize,
    pub cells_checked: usize,
    pub wal_records_checked: u64,
    pub wal_safe_truncate_offset: u64,
    pub errors: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct PutCellResponse {
    pub seq: u64,
    pub cell_id: u64,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct CellResponse {
    pub cell_id: u64,
    pub payload: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct CellLookupResponse {
    pub cell: Option<CellResponse>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct RememberResponse {
    pub seq: u64,
    pub cell_id: u64,
    pub ttl_seconds: Option<u64>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct DeleteJobResponse {
    pub deleted: bool,
}
