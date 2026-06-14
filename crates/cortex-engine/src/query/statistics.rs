use std::collections::BTreeSet;

use cortex_aql::{AqlCatalog, BitmapHandle, BrainId, CellTypeId, MemoryType, ScopeId, StatusId};
use cortex_storage::manifest::{ManifestCount, ManifestSegmentStats, StorageManifest};

use super::brain::resolve_single_brain_name;
use super::metadata::{
    cell_type_handle, cell_type_id, memory_type_handle, scope_handle, scope_id, status_handle,
    status_id,
};
use crate::database::Database;

mod segment_pruning;
mod zone_maps;

#[derive(Clone, Copy, Debug)]
pub struct DatabaseStatistics<'a> {
    manifest: &'a StorageManifest,
}

impl<'a> DatabaseStatistics<'a> {
    pub(crate) fn new(manifest: &'a StorageManifest) -> Self {
        Self { manifest }
    }

    pub fn live_segment_row_count(&self) -> u64 {
        let live_ids = self.live_segment_ids();
        let stats_total = self
            .live_segment_stats(&live_ids)
            .map(|stats| stats.row_count)
            .sum::<u64>();
        if stats_total > 0 || !self.manifest.segment_stats.is_empty() {
            stats_total
        } else {
            self.manifest
                .live_segments
                .iter()
                .map(|segment| u64::from(segment.cell_count))
                .sum()
        }
    }

    pub fn estimate_scope_cardinality(&self, scope: &str) -> Option<u64> {
        self.estimate_count(|stats| &stats.scope_counts, scope)
    }

    pub fn estimate_status_cardinality(&self, status: &str) -> Option<u64> {
        self.estimate_count(|stats| &stats.status_counts, status)
    }

    pub fn estimate_cell_type_cardinality(&self, cell_type: &str) -> Option<u64> {
        self.estimate_count(|stats| &stats.type_counts, cell_type)
    }

    pub fn estimate_term_document_frequency(&self, term: &str) -> Option<u64> {
        let live_ids = self.live_segment_ids();
        let mut saw_stats = false;
        let total = self
            .live_segment_stats(&live_ids)
            .map(|stats| {
                saw_stats = true;
                stats
                    .top_terms
                    .iter()
                    .find(|frequency| frequency.term == term)
                    .map(|frequency| frequency.document_frequency)
                    .unwrap_or_default()
            })
            .sum::<u64>();
        (saw_stats && total > 0).then_some(total)
    }

    pub fn estimate_bitmap_cardinality(&self, handle: BitmapHandle) -> Option<u64> {
        self.estimate_bitmap_count(
            handle,
            |stats| &stats.scope_counts,
            |key| scope_handle(scope_id(key)),
        )
        .or_else(|| {
            self.estimate_bitmap_count(
                handle,
                |stats| &stats.status_counts,
                |key| status_handle(status_id(key)),
            )
        })
        .or_else(|| {
            self.estimate_bitmap_count(
                handle,
                |stats| &stats.type_counts,
                |key| cell_type_handle(cell_type_id(key)),
            )
        })
    }

    fn estimate_bitmap_count(
        &self,
        handle: BitmapHandle,
        counts_for_stats: impl Fn(&ManifestSegmentStats) -> &[ManifestCount],
        handle_for_key: impl Fn(&str) -> BitmapHandle,
    ) -> Option<u64> {
        let live_ids = self.live_segment_ids();
        let mut saw_stats = false;
        let mut matched = false;
        let total = self
            .live_segment_stats(&live_ids)
            .map(|stats| {
                saw_stats = true;
                if let Some(count) =
                    sum_matching_counts(counts_for_stats(stats), handle, &handle_for_key)
                {
                    matched = true;
                    count
                } else {
                    0
                }
            })
            .sum::<u64>();
        (saw_stats && matched).then_some(total)
    }

    fn estimate_count(
        &self,
        counts_for_stats: impl Fn(&ManifestSegmentStats) -> &[ManifestCount],
        key: &str,
    ) -> Option<u64> {
        let live_ids = self.live_segment_ids();
        let mut saw_stats = false;
        let total = self
            .live_segment_stats(&live_ids)
            .map(|stats| {
                saw_stats = true;
                count_for(counts_for_stats(stats), key).unwrap_or_default()
            })
            .sum::<u64>();
        saw_stats.then_some(total)
    }

    fn live_segment_ids(&self) -> BTreeSet<u64> {
        self.manifest
            .live_segments
            .iter()
            .map(|segment| segment.id)
            .collect()
    }

    fn live_segment_stats<'b>(
        &'b self,
        live_ids: &'b BTreeSet<u64>,
    ) -> impl Iterator<Item = &'b ManifestSegmentStats> + 'b {
        self.manifest
            .segment_stats
            .iter()
            .filter(|stats| live_ids.contains(&stats.segment_id))
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct EngineAqlStatsCatalog<'a> {
    statistics: DatabaseStatistics<'a>,
}

impl<'a> EngineAqlStatsCatalog<'a> {
    pub(crate) fn new(statistics: DatabaseStatistics<'a>) -> Self {
        Self { statistics }
    }
}

impl AqlCatalog for EngineAqlStatsCatalog<'_> {
    fn resolve_brain(&self, name: &str) -> Option<BrainId> {
        resolve_single_brain_name(name)
    }

    fn resolve_scope(&self, _brain: BrainId, name: &str) -> Option<ScopeId> {
        Some(scope_id(name))
    }

    fn resolve_write_scope(&self, name: &str) -> Option<ScopeId> {
        Some(scope_id(name))
    }

    fn resolve_status(&self, _brain: BrainId, status: &str) -> Option<StatusId> {
        Some(status_id(status))
    }

    fn resolve_cell_type(&self, _brain: BrainId, cell_type: &str) -> Option<CellTypeId> {
        Some(cell_type_id(cell_type))
    }

    fn scope_bitmap(&self, _brain: BrainId, scope: ScopeId) -> Option<BitmapHandle> {
        Some(scope_handle(scope))
    }

    fn status_bitmap(&self, _brain: BrainId, status: StatusId) -> Option<BitmapHandle> {
        Some(status_handle(status))
    }

    fn cell_type_bitmap(&self, _brain: BrainId, cell_type: CellTypeId) -> Option<BitmapHandle> {
        Some(cell_type_handle(cell_type))
    }

    fn memory_type_bitmap(&self, _brain: BrainId, memory_type: MemoryType) -> Option<BitmapHandle> {
        Some(memory_type_handle(memory_type))
    }

    fn field_is_filterable(&self, _brain: BrainId, field: &str) -> bool {
        matches!(
            field,
            "space" | "scope" | "status" | "type" | "cell_type" | "memory_type"
        )
    }

    fn bitmap_estimated_cardinality(&self, _brain: BrainId, handle: BitmapHandle) -> Option<u64> {
        self.statistics.estimate_bitmap_cardinality(handle)
    }
}

impl Database {
    pub fn statistics(&self) -> DatabaseStatistics<'_> {
        DatabaseStatistics::new(self.manifest())
    }

    pub(crate) fn aql_statistics_catalog(&self) -> EngineAqlStatsCatalog<'_> {
        EngineAqlStatsCatalog::new(self.statistics())
    }
}

fn count_for(counts: &[ManifestCount], key: &str) -> Option<u64> {
    counts
        .iter()
        .find(|count| count.key == key)
        .map(|count| count.count)
}

fn sum_matching_counts(
    counts: &[ManifestCount],
    handle: BitmapHandle,
    handle_for_key: impl Fn(&str) -> BitmapHandle,
) -> Option<u64> {
    let mut matched = false;
    let mut total = 0u64;
    for count in counts {
        if handle_for_key(&count.key) == handle {
            matched = true;
            total = total.saturating_add(count.count);
        }
    }
    matched.then_some(total)
}

#[cfg(test)]
mod tests;
