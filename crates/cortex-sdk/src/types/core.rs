use serde::{Deserialize, Serialize};

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

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct WriteBatchResponse {
    pub seq: u64,
    pub operation_count: usize,
    pub cell_ids: Vec<u64>,
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
