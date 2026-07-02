use std::collections::{BTreeMap, BTreeSet};

mod brain;
pub(crate) mod cache;
mod candidates;
mod catalog;
pub(crate) mod delta;
mod explain;
mod index;
mod index_merge;
pub(crate) mod metadata;
mod metadata_validation;
mod provider;
mod pruning;
mod render;
mod statistics;
mod verify_explain;
use cortex_aql::{AgentView, BitmapHandle, BoundPlan, BoundRetrievePlan};
use cortex_core::CellId;

use crate::database::{Database, RetrievedCell};
use crate::error::{EngineError, EngineResult};
use crate::feedback::current_unix_seconds;
pub use cache::AqlQueryCacheStats;
pub(crate) use delta::AqlDeltaIndex;
pub use explain::{
    AqlCandidateCounts, AqlExecutionTraceReport, AqlExplainFilter, AqlExplainReport,
};
pub use metadata::{lexical_field_weight, scope_id, CellMetadata, SourceRef};
pub use provider::EngineAqlProvider;
pub use statistics::DatabaseStatistics;
pub use verify_explain::AqlVerifyExplainReport;

#[derive(Clone, Debug, Default)]
pub struct EngineAqlIndex {
    pub bitmaps: BTreeMap<BitmapHandle, BTreeSet<u32>>,
    pub lexical: BTreeMap<String, BTreeSet<u32>>,
    pub lexical_doc_lengths: BTreeMap<u32, u32>,
    pub lexical_term_frequencies: BTreeMap<String, BTreeMap<u32, u32>>,
    pub lexical_field_doc_lengths: BTreeMap<String, BTreeMap<u32, u32>>,
    pub lexical_field_term_frequencies: BTreeMap<String, BTreeMap<String, BTreeMap<u32, u32>>>,
    pub universe: BTreeSet<u32>,
    pub candidate_to_cell: BTreeMap<u32, CellId>,
    pub cell_to_candidate: BTreeMap<CellId, u32>,
    pub permission_pruning: AqlPermissionPruning,
    pub segment_pruning: AqlSegmentPruning,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CandidateId(pub u32);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AqlPermissionPruning {
    pub total_segments: usize,
    pub opened_segments: usize,
    pub skipped_segments: usize,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AqlSegmentPruning {
    pub total_segments: usize,
    pub opened_segments: usize,
    pub skipped_segments: usize,
}

impl Database {
    pub fn aql_index(&self) -> EngineResult<EngineAqlIndex> {
        self.try_aql_index()
    }

    pub fn try_aql_index(&self) -> EngineResult<EngineAqlIndex> {
        let analyzer = self.text_analyzer();
        if self.manifest().live_segments.is_empty() {
            return EngineAqlIndex::try_from_delta_with_analyzer(&self.aql_delta_index, &analyzer);
        }
        let persisted = self.persisted_index_state_cached()?;
        EngineAqlIndex::from_persisted_delta_with_analyzer(
            persisted.bitmap.clone(),
            persisted.lexical.clone(),
            persisted.candidate_to_cell.clone(),
            &self.aql_delta_index,
            &analyzer,
        )
    }

    pub(crate) fn try_aql_index_for_bound_plan(
        &self,
        view: &AgentView,
        bound_plan: &BoundPlan,
    ) -> EngineResult<EngineAqlIndex> {
        let retrieve_plan = match bound_plan {
            BoundPlan::Retrieve(plan) => Some(plan.as_ref()),
            _ => None,
        };
        self.try_aql_index_for_retrieve_plan(view, retrieve_plan)
    }

    fn try_aql_index_for_retrieve_plan(
        &self,
        view: &AgentView,
        retrieve_plan: Option<&BoundRetrievePlan>,
    ) -> EngineResult<EngineAqlIndex> {
        let analyzer = self.text_analyzer();
        let readable_scopes = crate::plan::PolicyRewrite::new(view)
            .readable_scopes()
            .collect::<BTreeSet<_>>();
        let mut index = if self.manifest().live_segments.is_empty() {
            EngineAqlIndex::try_from_delta_with_analyzer(&self.aql_delta_index, &analyzer)?
        } else {
            let total_segments = self.manifest().live_segments.len();
            let permission_segments = self
                .statistics()
                .segments_matching_any_scope_id(&readable_scopes);
            let segment_ids = retrieve_plan
                .map(|plan| &plan.bitmap_program)
                .and_then(|program| {
                    self.statistics()
                        .segments_matching_bitmap_program(program, &readable_scopes)
                })
                .or_else(|| permission_segments.clone());
            let segment_ids = intersect_segment_selection(
                segment_ids,
                retrieve_plan.and_then(|plan| self.freshness_segment_selection(plan)),
            );
            let permission_pruning =
                permission_pruning_report(total_segments, &permission_segments);
            let segment_pruning = segment_pruning_report(total_segments, &segment_ids);
            let persisted = match &segment_ids {
                Some(segment_ids) if segment_ids.len() < total_segments => {
                    let segment_ids = segment_ids.iter().copied().collect::<BTreeSet<_>>();
                    self.persisted_index_state_for_segments(&segment_ids)?
                }
                _ => (*self.persisted_index_state_cached()?).clone(),
            };
            let mut index = EngineAqlIndex::from_persisted_delta_with_analyzer(
                persisted.bitmap,
                persisted.lexical,
                persisted.candidate_to_cell,
                &self.aql_delta_index,
                &analyzer,
            )?;
            index.permission_pruning = permission_pruning;
            index.segment_pruning = segment_pruning;
            index
        };
        index.prune_to_readable_scopes(&readable_scopes);
        Ok(index)
    }

    pub fn retrieve_aql(&self, aql: &str, view: &AgentView) -> EngineResult<Vec<RetrievedCell>> {
        let (cached, index) = self.bind_aql_cached(aql, view)?;
        if cached.statement_kind != cache::AqlStatementKind::Retrieve {
            return Err(EngineError::InvalidOperation);
        }
        match cached.bound_plan {
            BoundPlan::Retrieve(plan) => {
                let provider = EngineAqlProvider::new(index, view);
                self.retrieve_cells(&plan, &provider)
            }
            _ => Err(EngineError::InvalidOperation),
        }
    }

    pub fn retrieve_aql_with_allowed_cells(
        &self,
        aql: &str,
        view: &AgentView,
        allowed_cells: &BTreeSet<CellId>,
    ) -> EngineResult<Vec<RetrievedCell>> {
        let (cached, index) = self.bind_aql_cached(aql, view)?;
        if cached.statement_kind != cache::AqlStatementKind::Retrieve {
            return Err(EngineError::InvalidOperation);
        }
        let allowed_candidates = allowed_cells
            .iter()
            .filter_map(|cell_id| index.cell_to_candidate.get(cell_id).copied())
            .collect::<BTreeSet<_>>();
        match cached.bound_plan {
            BoundPlan::Retrieve(plan) => {
                let provider = EngineAqlProvider::new_with_allowed_candidates(
                    index,
                    view,
                    &allowed_candidates,
                );
                self.retrieve_cells(&plan, &provider)
            }
            _ => Err(EngineError::InvalidOperation),
        }
    }
}

fn permission_pruning_report(
    total_segments: usize,
    segment_ids: &Option<Vec<u64>>,
) -> AqlPermissionPruning {
    let opened_segments = segment_ids
        .as_ref()
        .map_or(total_segments, std::vec::Vec::len);
    AqlPermissionPruning {
        total_segments,
        opened_segments,
        skipped_segments: total_segments.saturating_sub(opened_segments),
    }
}

fn segment_pruning_report(
    total_segments: usize,
    segment_ids: &Option<Vec<u64>>,
) -> AqlSegmentPruning {
    let opened_segments = segment_ids
        .as_ref()
        .map_or(total_segments, std::vec::Vec::len);
    AqlSegmentPruning {
        total_segments,
        opened_segments,
        skipped_segments: total_segments.saturating_sub(opened_segments),
    }
}

fn intersect_segment_selection(lhs: Option<Vec<u64>>, rhs: Option<Vec<u64>>) -> Option<Vec<u64>> {
    match (lhs, rhs) {
        (Some(lhs), Some(rhs)) => {
            let lhs = lhs.into_iter().collect::<BTreeSet<_>>();
            Some(rhs.into_iter().filter(|id| lhs.contains(id)).collect())
        }
        (Some(ids), None) | (None, Some(ids)) => Some(ids),
        (None, None) => None,
    }
}

impl Database {
    fn freshness_segment_selection(&self, plan: &BoundRetrievePlan) -> Option<Vec<u64>> {
        let max_freshness_seconds = plan.quality_thresholds.max_freshness_seconds?;
        let min_created = current_unix_seconds().saturating_sub(max_freshness_seconds);
        self.statistics()
            .segments_overlapping_created_range(Some(min_created), None)
    }
}
