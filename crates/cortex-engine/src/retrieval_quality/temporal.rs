use crate::query::CellMetadata;
use crate::verification::temporal::{
    parse_temporal_date, temporal_validity_from_metadata, TemporalQueryRange,
};

pub(super) fn metadata_is_valid_at(metadata: &CellMetadata, valid_at: Option<&str>) -> bool {
    let Some(valid_at) = valid_at else {
        return true;
    };
    let Some(valid_at) = parse_temporal_date(valid_at) else {
        return false;
    };
    let query = TemporalQueryRange {
        start: valid_at,
        end: valid_at,
    };
    temporal_validity_from_metadata(metadata)
        .stale_reason(query)
        .is_none()
}
