use cortex_aql::QualityThresholds;
use cortex_core::memtable::CellVersion;

use crate::query::CellMetadata;

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

    if let Some(max_freshness_seconds) = thresholds.max_freshness_seconds {
        let Some(created) = metadata.created_unix_seconds else {
            return false;
        };
        let age = unix_now_seconds().saturating_sub(created);
        if age > max_freshness_seconds {
            return false;
        }
    }

    true
}

pub(crate) fn cell_version_meets_quality_thresholds(
    version: &CellVersion,
    thresholds: &QualityThresholds,
) -> bool {
    metadata_meets_quality_thresholds(&CellMetadata::from_version(version), thresholds)
}

fn unix_now_seconds() -> u64 {
    match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
        Ok(duration) => duration.as_secs(),
        Err(_) => 0,
    }
}
