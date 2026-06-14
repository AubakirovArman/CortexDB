use cortex_aql::QualityThresholds;
use cortex_core::memtable::CellVersion;

mod checks;
mod temporal;
mod validity_index;

pub(crate) fn cell_version_meets_quality_thresholds(
    version: &CellVersion,
    thresholds: &QualityThresholds,
) -> bool {
    checks::cell_version_meets_quality_thresholds(version, thresholds)
}

pub(crate) use validity_index::TemporalValidityStore;
