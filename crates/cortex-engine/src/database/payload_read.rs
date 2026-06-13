use cortex_core::memtable::{CellVersion, PayloadRef};
use cortex_storage::segment::SegmentReader;

use super::payload_cache::SegmentPayloadCacheKey;
use super::Database;
use crate::checkpoint::segment_path;
use crate::error::{EngineError, EngineResult};

impl Database {
    pub(crate) fn payload_for_version(&self, version: &CellVersion) -> EngineResult<Vec<u8>> {
        match &version.payload_ref {
            PayloadRef::Inline => Ok(version.payload.clone()),
            PayloadRef::Segment {
                segment_id,
                candidate_id,
                ..
            } => {
                let key = SegmentPayloadCacheKey::new(*segment_id, *candidate_id);
                if let Some(payload) = self
                    .payload_cache
                    .lock()
                    .expect("payload cache lock poisoned")
                    .get(key)
                {
                    return Ok(payload);
                }

                let payload =
                    read_segment_payload(&self.segments_path, *segment_id, *candidate_id)?;
                let mut cache = self
                    .payload_cache
                    .lock()
                    .expect("payload cache lock poisoned");
                cache.record_segment_load();
                cache.insert(key, payload.clone());
                Ok(payload)
            }
        }
    }

    pub(crate) fn payload_for_version_uncached(
        &self,
        version: &CellVersion,
    ) -> EngineResult<Vec<u8>> {
        match &version.payload_ref {
            PayloadRef::Inline => Ok(version.payload.clone()),
            PayloadRef::Segment {
                segment_id,
                candidate_id,
                ..
            } => read_segment_payload(&self.segments_path, *segment_id, *candidate_id),
        }
    }
}

fn read_segment_payload(
    segments_path: &std::path::Path,
    segment_id: u64,
    candidate_id: u32,
) -> EngineResult<Vec<u8>> {
    SegmentReader::read_payload_at(segment_path(segments_path, segment_id), candidate_id)?
        .ok_or_else(|| {
            EngineError::StorageInvariant(format!(
                "segment {segment_id} is missing payload for candidate {candidate_id}"
            ))
        })
}
