use crate::database::RetrievedCell;
use crate::query::CellMetadata;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SourceFreshnessCategory {
    Unknown,
    Stale,
    Older,
    Recent,
    Current,
}

impl SourceFreshnessCategory {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Unknown => "unknown",
            Self::Stale => "stale",
            Self::Older => "older",
            Self::Recent => "recent",
            Self::Current => "current",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SourceFreshness {
    pub q16: u16,
    pub category: SourceFreshnessCategory,
    pub has_timestamp: bool,
}

impl SourceFreshness {
    pub fn from_created_unix_seconds(
        created_unix_seconds: Option<u64>,
        range: SourceFreshnessRange,
    ) -> Self {
        let Some(created) = created_unix_seconds else {
            return Self {
                q16: 0,
                category: SourceFreshnessCategory::Unknown,
                has_timestamp: false,
            };
        };

        let q16 = match (
            range.min_created_unix_seconds,
            range.max_created_unix_seconds,
        ) {
            (Some(min), Some(max)) if max > min => {
                let age_span = max - min;
                let relative = created.saturating_sub(min).min(age_span);
                ((relative * u64::from(u16::MAX)) / age_span) as u16
            }
            _ => u16::MAX,
        };

        Self {
            q16,
            category: category_for_q16(q16),
            has_timestamp: true,
        }
    }

    pub fn score_bonus(self) -> u32 {
        u32::from(self.q16) / 2
    }

    pub fn score_reason(self) -> String {
        if self.has_timestamp {
            format!(
                "{} source freshness from created_unix_seconds relative to retrieved candidates",
                self.category.as_str()
            )
        } else {
            "no source freshness because created_unix_seconds metadata is absent".to_owned()
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SourceFreshnessRange {
    min_created_unix_seconds: Option<u64>,
    max_created_unix_seconds: Option<u64>,
}

impl SourceFreshnessRange {
    pub fn from_cells(cells: &[RetrievedCell]) -> Self {
        let mut min_created_unix_seconds = None;
        let mut max_created_unix_seconds = None;
        for cell in cells {
            let Some(created) = CellMetadata::from_payload(&cell.payload).created_unix_seconds
            else {
                continue;
            };
            min_created_unix_seconds = Some(
                min_created_unix_seconds
                    .map(|current: u64| current.min(created))
                    .unwrap_or(created),
            );
            max_created_unix_seconds = Some(
                max_created_unix_seconds
                    .map(|current: u64| current.max(created))
                    .unwrap_or(created),
            );
        }
        Self {
            min_created_unix_seconds,
            max_created_unix_seconds,
        }
    }
}

fn category_for_q16(q16: u16) -> SourceFreshnessCategory {
    match q16 {
        0..=16_383 => SourceFreshnessCategory::Stale,
        16_384..=39_321 => SourceFreshnessCategory::Older,
        39_322..=58_981 => SourceFreshnessCategory::Recent,
        _ => SourceFreshnessCategory::Current,
    }
}

#[cfg(test)]
mod tests {
    use cortex_core::CellId;

    use super::*;

    #[test]
    fn missing_timestamp_has_no_freshness_bonus() {
        let freshness =
            SourceFreshness::from_created_unix_seconds(None, SourceFreshnessRange::from_cells(&[]));

        assert_eq!(freshness.q16, 0);
        assert_eq!(freshness.category, SourceFreshnessCategory::Unknown);
        assert_eq!(freshness.score_bonus(), 0);
    }

    #[test]
    fn newest_timestamp_is_current_relative_to_candidate_set() {
        let range = SourceFreshnessRange::from_cells(&[
            retrieved(1, "created_unix_seconds=10\n\nold"),
            retrieved(2, "created_unix_seconds=20\n\nnew"),
        ]);

        let old = SourceFreshness::from_created_unix_seconds(Some(10), range);
        let new = SourceFreshness::from_created_unix_seconds(Some(20), range);

        assert_eq!(old.category, SourceFreshnessCategory::Stale);
        assert_eq!(new.q16, u16::MAX);
        assert_eq!(new.category, SourceFreshnessCategory::Current);
        assert!(new.score_bonus() > old.score_bonus());
    }

    fn retrieved(cell_id: u64, payload: &str) -> RetrievedCell {
        RetrievedCell {
            cell_id: CellId(cell_id),
            payload: payload.as_bytes().to_vec(),
        }
    }
}
