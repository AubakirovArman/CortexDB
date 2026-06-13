use std::collections::BTreeSet;
use std::fs;
use std::io::ErrorKind;
use std::path::Path;

use crate::atomic::write_atomic;
use crate::error::StorageResult;

use self::codec::{decode_manifest, encode_manifest};

mod codec;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ManifestSegment {
    pub id: u64,
    pub generation: u64,
    pub checkpoint_seq: u64,
    pub cell_count: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ManifestHnswProfile {
    pub max_neighbors: u32,
    pub ef_search: u32,
    pub layer_count: u32,
    pub metric: u32,
    pub ef_construction: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ManifestVectorProfile {
    pub dimension: u32,
    pub metric: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ManifestCount {
    pub key: String,
    pub count: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ManifestTermDocumentFrequency {
    pub term: String,
    pub document_frequency: u64,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ManifestSegmentStats {
    pub segment_id: u64,
    pub row_count: u64,
    pub min_created_unix_seconds: Option<u64>,
    pub max_created_unix_seconds: Option<u64>,
    pub scope_counts: Vec<ManifestCount>,
    pub status_counts: Vec<ManifestCount>,
    pub type_counts: Vec<ManifestCount>,
    pub top_terms: Vec<ManifestTermDocumentFrequency>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ManifestHnswNoFallbackProfile {
    pub rollout_enabled: bool,
    pub min_recall_q16: u16,
    pub require_upper_layers: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CompactionMetadata {
    pub triggered: u64,
    pub completed: u64,
    pub duration_ms: u64,
    pub cells_compacted: u64,
    pub input_bytes: u64,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StorageManifest {
    pub generation: u64,
    pub checkpoint_seq: u64,
    pub live_segments: Vec<ManifestSegment>,
    pub retired_segments: Vec<ManifestSegment>,
    pub hnsw_profile: Option<ManifestHnswProfile>,
    pub vector_profile: Option<ManifestVectorProfile>,
    pub segment_stats: Vec<ManifestSegmentStats>,
    pub hnsw_no_fallback_profile: Option<ManifestHnswNoFallbackProfile>,
    pub compaction_metadata: CompactionMetadata,
}

impl StorageManifest {
    pub fn load(path: impl AsRef<Path>) -> StorageResult<Self> {
        match fs::read(path) {
            Ok(bytes) => decode_manifest(&bytes),
            Err(error) if error.kind() == ErrorKind::NotFound => Ok(Self::default()),
            Err(error) => Err(error.into()),
        }
    }

    pub fn store(&self, path: impl AsRef<Path>) -> StorageResult<()> {
        write_atomic(path.as_ref(), &encode_manifest(self))?;
        Ok(())
    }

    pub fn checkpoint_segment(&mut self, segment: ManifestSegment) {
        self.generation += 1;
        self.checkpoint_seq = segment.checkpoint_seq;
        self.live_segments.push(segment);
    }

    pub fn compact_to_segment(&mut self, segment: ManifestSegment) {
        self.generation += 1;
        self.checkpoint_seq = segment.checkpoint_seq;
        let retired_ids = self
            .live_segments
            .iter()
            .map(|segment| segment.id)
            .collect::<BTreeSet<_>>();
        self.retired_segments.append(&mut self.live_segments);
        self.live_segments.push(segment);
        self.segment_stats
            .retain(|stats| !retired_ids.contains(&stats.segment_id));
    }

    /// Replace a contiguous subset of live segments with a single merged segment.
    /// The replaced segments are appended to `retired_segments`. The checkpoint
    /// sequence is not changed: the merge does not advance the WAL truncation
    /// horizon.
    pub fn replace_segments(&mut self, selected: Vec<ManifestSegment>, merged: ManifestSegment) {
        self.generation += 1;
        let selected_ids: BTreeSet<u64> = selected.iter().map(|segment| segment.id).collect();
        let mut new_live = Vec::with_capacity(self.live_segments.len() - selected.len() + 1);
        // The selected segments are expected to be the oldest contiguous live
        // segments, so the merged segment takes their place at the front.
        new_live.push(merged);
        for segment in &self.live_segments {
            if !selected_ids.contains(&segment.id) {
                new_live.push(segment.clone());
            }
        }
        self.live_segments = new_live;
        self.retired_segments.extend(selected);
        self.segment_stats
            .retain(|stats| !selected_ids.contains(&stats.segment_id));
    }

    pub fn stats_for_segment(&self, segment_id: u64) -> Option<&ManifestSegmentStats> {
        self.segment_stats
            .iter()
            .find(|stats| stats.segment_id == segment_id)
    }

    pub fn set_segment_stats(&mut self, stats: ManifestSegmentStats) {
        self.segment_stats
            .retain(|existing| existing.segment_id != stats.segment_id);
        self.segment_stats.push(stats);
        self.segment_stats.sort_by_key(|stats| stats.segment_id);
    }
}
