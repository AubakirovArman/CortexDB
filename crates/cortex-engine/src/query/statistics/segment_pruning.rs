use std::collections::BTreeSet;

use cortex_aql::{BitmapHandle, BitmapOp, BitmapProgram, ScopeId};
use cortex_storage::manifest::{ManifestCount, ManifestSegmentStats};

use super::DatabaseStatistics;
use crate::query::metadata::{
    bitmap_handle_kind, cell_type_handle, cell_type_id, scope_handle, scope_id, status_handle,
    status_id, BitmapHandleKind,
};

impl DatabaseStatistics<'_> {
    pub(crate) fn segments_matching_bitmap_program(
        &self,
        program: &BitmapProgram,
        readable_scopes: &BTreeSet<ScopeId>,
    ) -> Option<Vec<u64>> {
        if self.manifest.live_segments.is_empty() {
            return Some(Vec::new());
        }
        if !self.has_live_segment_stats() {
            return None;
        }

        let all_segments = self.live_segment_id_set();
        let mut stack = Vec::<Option<BTreeSet<u64>>>::new();
        for op in &program.ops {
            match op {
                BitmapOp::Push(handle) => {
                    stack.push(self.segments_matching_bitmap_handle(*handle));
                }
                BitmapOp::PushAgentAllowed => {
                    stack.push(
                        self.segments_matching_any_scope_id(readable_scopes)
                            .map(|ids| ids.into_iter().collect()),
                    );
                }
                BitmapOp::PushUniverse | BitmapOp::PushLive => stack.push(None),
                BitmapOp::And => {
                    let rhs = stack.pop()?;
                    let lhs = stack.pop()?;
                    stack.push(intersect_segments(lhs, rhs));
                }
                BitmapOp::Or => {
                    let rhs = stack.pop()?;
                    let lhs = stack.pop()?;
                    stack.push(union_segments(lhs, rhs));
                }
                BitmapOp::Not => {
                    stack.pop()?;
                    stack.push(None);
                }
            }
        }
        let [segments] = stack.as_slice() else {
            return None;
        };
        segments
            .clone()
            .map(|segments| segments.into_iter().collect())
            .or_else(|| Some(all_segments.into_iter().collect()))
    }

    fn segments_matching_bitmap_handle(&self, handle: BitmapHandle) -> Option<BTreeSet<u64>> {
        match bitmap_handle_kind(handle) {
            BitmapHandleKind::Scope => self.segments_matching_handle_count(
                |stats| &stats.scope_counts,
                |key| scope_handle(scope_id(key)),
                handle,
            ),
            BitmapHandleKind::Status => self.segments_matching_handle_count(
                |stats| &stats.status_counts,
                |key| status_handle(status_id(key)),
                handle,
            ),
            BitmapHandleKind::CellType => self.segments_matching_handle_count(
                |stats| &stats.type_counts,
                |key| cell_type_handle(cell_type_id(key)),
                handle,
            ),
            BitmapHandleKind::MemoryType | BitmapHandleKind::Unknown => None,
        }
    }

    fn segments_matching_handle_count(
        &self,
        counts_for_stats: impl Fn(&ManifestSegmentStats) -> &[ManifestCount],
        handle_for_key: impl Fn(&str) -> BitmapHandle,
        handle: BitmapHandle,
    ) -> Option<BTreeSet<u64>> {
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
                    segment_may_contain_handle(
                        stats,
                        counts_for_stats(stats),
                        &handle_for_key,
                        handle,
                    )
                    .then_some(segment.id)
                })
                .collect(),
        )
    }

    fn live_segment_id_set(&self) -> BTreeSet<u64> {
        self.manifest
            .live_segments
            .iter()
            .map(|segment| segment.id)
            .collect()
    }
}

fn segment_may_contain_handle(
    stats: &ManifestSegmentStats,
    counts: &[ManifestCount],
    handle_for_key: &impl Fn(&str) -> BitmapHandle,
    handle: BitmapHandle,
) -> bool {
    let mut counted_rows = 0u64;
    for count in counts {
        counted_rows = counted_rows.saturating_add(count.count);
        if count.count > 0 && handle_for_key(&count.key) == handle {
            return true;
        }
    }
    counted_rows < stats.row_count
}

fn intersect_segments(
    lhs: Option<BTreeSet<u64>>,
    rhs: Option<BTreeSet<u64>>,
) -> Option<BTreeSet<u64>> {
    match (lhs, rhs) {
        (Some(lhs), Some(rhs)) => Some(lhs.intersection(&rhs).copied().collect()),
        (Some(segments), None) | (None, Some(segments)) => Some(segments),
        (None, None) => None,
    }
}

fn union_segments(lhs: Option<BTreeSet<u64>>, rhs: Option<BTreeSet<u64>>) -> Option<BTreeSet<u64>> {
    match (lhs, rhs) {
        (Some(lhs), Some(rhs)) => Some(lhs.union(&rhs).copied().collect()),
        _ => None,
    }
}
