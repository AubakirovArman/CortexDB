use cortex_core::CommitSeq;

use crate::database::Database;
use crate::error::EngineResult;
use crate::memory_accounting::estimate_database_memory;

use super::helpers::{file_len_or_zero, ratio_q16, segment_usage};
use super::StorageStats;

impl Database {
    pub fn storage_stats(&self) -> EngineResult<StorageStats> {
        let memory = estimate_database_memory(self)?;
        let live_usage = segment_usage(&self.segments_path, &self.manifest.live_segments, true)?;
        let retired_usage =
            segment_usage(&self.segments_path, &self.manifest.retired_segments, false)?;
        let total_segment_bytes = live_usage
            .bundle_bytes
            .saturating_add(retired_usage.bundle_bytes);
        let wal_size_bytes = file_len_or_zero(&self.wal_path)?;
        let wal_writer = self.writer.metrics()?;
        let durable_storage_bytes = total_segment_bytes.saturating_add(wal_size_bytes);
        let logical_payload_bytes = live_usage
            .payload_bytes
            .max(memory.memtable_payload_bytes as u64);
        let write_bytes = total_segment_bytes.saturating_add(wal_writer.bytes_written);
        Ok(StorageStats {
            current_seq: self.current_seq,
            checkpoint_seq: CommitSeq(self.manifest.checkpoint_seq),
            live_segments: self.manifest.live_segments.len(),
            retired_segments: self.manifest.retired_segments.len(),
            memtable: self.memtable.stats(),
            memtable_payload_bytes: memory.memtable_payload_bytes,
            estimated_memtable_bytes: memory.estimated_memtable_bytes,
            estimated_index_bytes: memory.estimated_index_bytes,
            estimated_context_pack_bytes: memory.estimated_context_pack_bytes,
            estimated_total_memory_bytes: memory.estimated_total_memory_bytes,
            live_segment_bytes: live_usage.bundle_bytes,
            retired_segment_bytes: retired_usage.bundle_bytes,
            total_segment_bytes,
            durable_storage_bytes,
            live_segment_payload_bytes: live_usage.payload_bytes,
            logical_payload_bytes,
            space_amplification_q16: ratio_q16(durable_storage_bytes, logical_payload_bytes),
            write_amplification_q16: ratio_q16(write_bytes, logical_payload_bytes),
            compaction_pressure_q16: ratio_q16(retired_usage.bundle_bytes, total_segment_bytes),
            wal_size_bytes,
            wal_writer,
            compactions_triggered: self.manifest.compaction_metadata.triggered,
            compactions_completed: self.manifest.compaction_metadata.completed,
            compaction_duration_ms_total: self.manifest.compaction_metadata.duration_ms,
            compaction_cells_compacted: self.manifest.compaction_metadata.cells_compacted,
            compaction_input_bytes: self.manifest.compaction_metadata.input_bytes,
        })
    }
}
