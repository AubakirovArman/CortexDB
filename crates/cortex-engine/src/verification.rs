mod conflict_index;
mod contradiction;
mod evidence;
mod execution;
pub mod export;
mod graph;
mod guards;
pub mod numeric;
mod operator;
mod support;
pub mod temporal;
mod temporal_index;
#[cfg(test)]
mod tests;
mod types;
pub(crate) use conflict_index::ConflictIndexStore;
pub use conflict_index::{ConflictRecord, ContradictionRelationOptions};
pub use export::VerificationReportExportFormat;
pub use numeric::{
    compare_numeric_values, format_scaled_value, normalized_numeric_equal, parse_currency_code,
    parse_magnitude_suffix, parse_unit_code, Magnitude, NumericComparison, NumericValue,
};
pub use temporal::{
    extract_temporal_query_range, parse_temporal_date, TemporalDate, TemporalQueryRange,
    TemporalStaleReason, TemporalValidity,
};
pub(crate) use temporal_index::TemporalFactStore;
pub use temporal_index::{TemporalFactIndex, TemporalFactKey, TemporalFactRecord};
pub use types::{
    VerificationEvidence, VerificationGuard, VerificationGuardCode, VerificationMatchKind,
    VerificationNumericConflict, VerificationReport, VerificationStatus,
};
