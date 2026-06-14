use cortex_aql::QualityThresholds;
use cortex_core::memtable::CellVersion;

use crate::query::CellMetadata;

pub(super) fn cell_version_meets_quality_thresholds(
    version: &CellVersion,
    thresholds: &QualityThresholds,
) -> bool {
    metadata_meets_quality_thresholds(&CellMetadata::from_version(version), thresholds)
}

fn metadata_meets_quality_thresholds(
    metadata: &CellMetadata,
    thresholds: &QualityThresholds,
) -> bool {
    let confidence_q16 = metadata
        .source_ref
        .as_ref()
        .map(|source_ref| source_ref.confidence_q16)
        .or(metadata.source_trust_q16)
        .unwrap_or(0);
    if confidence_q16 < thresholds.min_confidence_q16 {
        return false;
    }
    if metadata.source_trust_q16.unwrap_or(0) < thresholds.min_source_trust_q16 {
        return false;
    }
    if !meets_freshness(metadata, thresholds) {
        return false;
    }
    super::temporal::metadata_is_valid_at(metadata, thresholds.valid_at.as_deref())
}

fn meets_freshness(metadata: &CellMetadata, thresholds: &QualityThresholds) -> bool {
    let Some(max_freshness_seconds) = thresholds.max_freshness_seconds else {
        return true;
    };
    let Some(created) = metadata.created_unix_seconds else {
        return false;
    };
    unix_now_seconds().saturating_sub(created) <= max_freshness_seconds
}

fn unix_now_seconds() -> u64 {
    match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
        Ok(duration) => duration.as_secs(),
        Err(_) => 0,
    }
}
