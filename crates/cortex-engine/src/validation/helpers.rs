use std::fs;
use std::io::ErrorKind;
use std::path::Path;

use cortex_storage::manifest::ManifestSegment;
use cortex_storage::segment::SegmentReader;

use crate::checkpoint::{bitmap_path, hnsw_path, lexical_path, segment_path, vector_path};
use crate::error::EngineResult;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct SegmentUsage {
    pub(super) bundle_bytes: u64,
    pub(super) payload_bytes: u64,
}

pub(super) fn file_len_or_zero(path: &std::path::Path) -> EngineResult<u64> {
    match fs::metadata(path) {
        Ok(metadata) => Ok(metadata.len()),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(0),
        Err(error) => Err(error.into()),
    }
}

pub(super) fn segment_usage(
    segments_path: &Path,
    segments: &[ManifestSegment],
    read_payloads: bool,
) -> EngineResult<SegmentUsage> {
    let mut usage = SegmentUsage::default();
    for segment in segments {
        let segment_file = segment_path(segments_path, segment.id);
        let bundle_paths = [
            segment_file.clone(),
            bitmap_path(segments_path, segment.id),
            lexical_path(segments_path, segment.id),
            vector_path(segments_path, segment.id),
            hnsw_path(segments_path, segment.id),
        ];
        for path in bundle_paths {
            usage.bundle_bytes = usage.bundle_bytes.saturating_add(file_len_or_zero(&path)?);
        }
        if read_payloads {
            for cell in SegmentReader::read(&segment_file)? {
                if cell.deleted_seq.is_none() {
                    usage = SegmentUsage {
                        bundle_bytes: usage.bundle_bytes,
                        payload_bytes: usage
                            .payload_bytes
                            .saturating_add(cell.payload.len() as u64),
                    };
                }
            }
        }
    }
    Ok(usage)
}

pub(super) fn ratio_q16(numerator: u64, denominator: u64) -> u32 {
    if denominator == 0 {
        return 0;
    }
    let scaled = (u128::from(numerator) << 16) / u128::from(denominator);
    scaled.min(u128::from(u32::MAX)) as u32
}
