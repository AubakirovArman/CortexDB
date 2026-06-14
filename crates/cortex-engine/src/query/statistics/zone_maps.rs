use std::collections::BTreeSet;

use cortex_aql::ScopeId;
use cortex_storage::manifest::{ManifestCount, ManifestSegmentStats};

use super::{count_for, DatabaseStatistics};
use crate::query::metadata::scope_id;

impl DatabaseStatistics<'_> {
    pub fn segments_matching_scope(&self, scope: &str) -> Option<Vec<u64>> {
        self.segments_matching_count(|stats| &stats.scope_counts, scope)
    }

    pub(crate) fn segments_matching_any_scope_id(
        &self,
        scopes: &BTreeSet<ScopeId>,
    ) -> Option<Vec<u64>> {
        if self.manifest.live_segments.is_empty() {
            return Some(Vec::new());
        }
        if !self.has_live_segment_stats() {
            return None;
        }

        Some(
            self.manifest
                .live_segments
                .iter()
                .filter_map(|segment| {
                    let Some(stats) = self.manifest.stats_for_segment(segment.id) else {
                        return Some(segment.id);
                    };
                    segment_may_contain_scope_id(stats, scopes).then_some(segment.id)
                })
                .collect(),
        )
    }

    pub fn segments_matching_status(&self, status: &str) -> Option<Vec<u64>> {
        self.segments_matching_count(|stats| &stats.status_counts, status)
    }

    pub fn segments_matching_cell_type(&self, cell_type: &str) -> Option<Vec<u64>> {
        self.segments_matching_count(|stats| &stats.type_counts, cell_type)
    }

    pub fn segments_overlapping_created_range(
        &self,
        min_created_unix_seconds: Option<u64>,
        max_created_unix_seconds: Option<u64>,
    ) -> Option<Vec<u64>> {
        if self.manifest.live_segments.is_empty() {
            return Some(Vec::new());
        }
        if !self.has_live_segment_stats() {
            return None;
        }

        Some(
            self.manifest
                .live_segments
                .iter()
                .filter_map(|segment| {
                    let Some(stats) = self.manifest.stats_for_segment(segment.id) else {
                        return Some(segment.id);
                    };
                    created_range_overlaps(
                        stats.min_created_unix_seconds,
                        stats.max_created_unix_seconds,
                        min_created_unix_seconds,
                        max_created_unix_seconds,
                    )
                    .then_some(segment.id)
                })
                .collect(),
        )
    }

    fn segments_matching_count(
        &self,
        counts_for_stats: impl Fn(&ManifestSegmentStats) -> &[ManifestCount],
        key: &str,
    ) -> Option<Vec<u64>> {
        if self.manifest.live_segments.is_empty() {
            return Some(Vec::new());
        }
        if !self.has_live_segment_stats() {
            return None;
        }

        Some(
            self.manifest
                .live_segments
                .iter()
                .filter_map(|segment| {
                    let Some(stats) = self.manifest.stats_for_segment(segment.id) else {
                        return Some(segment.id);
                    };
                    (count_for(counts_for_stats(stats), key).unwrap_or_default() > 0)
                        .then_some(segment.id)
                })
                .collect(),
        )
    }

    pub(super) fn has_live_segment_stats(&self) -> bool {
        self.manifest
            .live_segments
            .iter()
            .any(|segment| self.manifest.stats_for_segment(segment.id).is_some())
    }
}

fn segment_may_contain_scope_id(stats: &ManifestSegmentStats, scopes: &BTreeSet<ScopeId>) -> bool {
    let mut scoped_rows = 0u64;
    for count in &stats.scope_counts {
        scoped_rows = scoped_rows.saturating_add(count.count);
        if count.count > 0 && scopes.contains(&scope_id(&count.key)) {
            return true;
        }
    }
    scoped_rows < stats.row_count
}

fn created_range_overlaps(
    segment_min: Option<u64>,
    segment_max: Option<u64>,
    query_min: Option<u64>,
    query_max: Option<u64>,
) -> bool {
    let (Some(segment_min), Some(segment_max)) = (segment_min, segment_max) else {
        return true;
    };
    if let Some(query_min) = query_min {
        if segment_max < query_min {
            return false;
        }
    }
    if let Some(query_max) = query_max {
        if segment_min > query_max {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use cortex_storage::manifest::{
        ManifestCount, ManifestSegment, ManifestSegmentStats, StorageManifest,
    };

    use super::*;

    #[test]
    fn zone_maps_filter_live_segments_without_retired_leaks() {
        let manifest = StorageManifest {
            live_segments: vec![
                manifest_segment(1),
                manifest_segment(2),
                manifest_segment(3),
            ],
            retired_segments: vec![manifest_segment(9)],
            segment_stats: vec![
                stats(
                    1,
                    &[("project:a", 2), ("project:b", 1)],
                    &[("ready", 1), ("draft", 2)],
                    &[("fact", 3)],
                    Some(10),
                    Some(20),
                ),
                stats(
                    2,
                    &[("project:c", 2)],
                    &[("ready", 2)],
                    &[("document_block", 2)],
                    Some(30),
                    Some(40),
                ),
                stats(
                    9,
                    &[("project:a", 100)],
                    &[("ready", 100)],
                    &[("fact", 100)],
                    Some(15),
                    Some(16),
                ),
            ],
            ..StorageManifest::default()
        };
        let statistics = DatabaseStatistics::new(&manifest);

        assert_eq!(
            statistics.segments_matching_scope("project:a"),
            Some(vec![1, 3])
        );
        assert_eq!(
            statistics.segments_matching_any_scope_id(&BTreeSet::from([scope_id("project:a")])),
            Some(vec![1, 3])
        );
        assert_eq!(
            statistics
                .segments_matching_any_scope_id(&BTreeSet::from([scope_id("project:missing")])),
            Some(vec![3])
        );
        assert_eq!(
            statistics.segments_matching_status("ready"),
            Some(vec![1, 2, 3])
        );
        assert_eq!(
            statistics.segments_matching_cell_type("document_block"),
            Some(vec![2, 3])
        );
        assert_eq!(
            statistics.segments_overlapping_created_range(Some(15), Some(35)),
            Some(vec![1, 2, 3])
        );
        assert_eq!(
            statistics.segments_overlapping_created_range(Some(21), Some(29)),
            Some(vec![3])
        );
    }

    #[test]
    fn zone_maps_are_unknown_when_no_live_segment_has_stats() {
        let manifest = StorageManifest {
            live_segments: vec![manifest_segment(1)],
            ..StorageManifest::default()
        };
        let statistics = DatabaseStatistics::new(&manifest);

        assert_eq!(statistics.segments_matching_scope("project:a"), None);
        assert_eq!(
            statistics.segments_overlapping_created_range(Some(1), Some(2)),
            None
        );
    }

    fn manifest_segment(id: u64) -> ManifestSegment {
        ManifestSegment {
            id,
            generation: id,
            checkpoint_seq: id,
            cell_count: 1,
        }
    }

    fn stats(
        segment_id: u64,
        scopes: &[(&str, u64)],
        statuses: &[(&str, u64)],
        types: &[(&str, u64)],
        min_created_unix_seconds: Option<u64>,
        max_created_unix_seconds: Option<u64>,
    ) -> ManifestSegmentStats {
        ManifestSegmentStats {
            segment_id,
            row_count: 1,
            min_created_unix_seconds,
            max_created_unix_seconds,
            scope_counts: counts(scopes),
            status_counts: counts(statuses),
            type_counts: counts(types),
            ..ManifestSegmentStats::default()
        }
    }

    fn counts(values: &[(&str, u64)]) -> Vec<ManifestCount> {
        values
            .iter()
            .map(|(key, count)| ManifestCount {
                key: (*key).to_owned(),
                count: *count,
            })
            .collect()
    }
}
