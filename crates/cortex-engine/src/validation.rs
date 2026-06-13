mod helpers;
mod issue;
mod report;
mod stats;

use cortex_core::memtable::MemTableStats;
use cortex_core::CommitSeq;
use cortex_storage::wal::WalWriterMetrics;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StorageStats {
    pub current_seq: CommitSeq,
    pub checkpoint_seq: CommitSeq,
    pub live_segments: usize,
    pub retired_segments: usize,
    pub memtable: MemTableStats,
    pub memtable_payload_bytes: usize,
    pub estimated_memtable_bytes: usize,
    pub estimated_index_bytes: usize,
    pub estimated_context_pack_bytes: usize,
    pub estimated_total_memory_bytes: usize,
    pub live_segment_bytes: u64,
    pub retired_segment_bytes: u64,
    pub total_segment_bytes: u64,
    pub durable_storage_bytes: u64,
    pub live_segment_payload_bytes: u64,
    pub logical_payload_bytes: u64,
    pub space_amplification_q16: u32,
    pub write_amplification_q16: u32,
    pub compaction_pressure_q16: u32,
    pub wal_size_bytes: u64,
    pub wal_writer: WalWriterMetrics,
    pub compactions_triggered: u64,
    pub compactions_completed: u64,
    pub compaction_duration_ms_total: u64,
    pub compaction_cells_compacted: u64,
    pub compaction_input_bytes: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StorageValidation {
    pub live_segments_checked: usize,
    pub cells_checked: usize,
    pub wal_records_checked: usize,
    pub wal_safe_truncate_offset: u64,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StorageValidationReport {
    pub manifest_ok: bool,
    pub wal_ok: bool,
    pub live_segments_checked: usize,
    pub bitmap_indexes_checked: usize,
    pub lexical_indexes_checked: usize,
    pub vector_indexes_checked: usize,
    pub hnsw_graphs_checked: usize,
    pub cells_checked: usize,
    pub wal_records_checked: usize,
    pub wal_safe_truncate_offset: u64,
    pub errors: Vec<String>,
    pub issues: Vec<StorageValidationIssue>,
}
pub use issue::{StorageRecoveryAction, StorageValidationIssue, StorageValidationIssueKind};
