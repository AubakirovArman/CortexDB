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
use cortex_aql::{AgentView, BitmapHandle, BoundPlan};
use cortex_core::CellId;

use crate::database::{Database, RetrievedCell};
use crate::error::{EngineError, EngineResult};
pub use cache::AqlQueryCacheStats;
pub(crate) use delta::AqlDeltaIndex;
pub use explain::{
    AqlCandidateCounts, AqlExecutionTraceReport, AqlExplainFilter, AqlExplainReport,
};
pub use metadata::{scope_id, CellMetadata, SourceRef};
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
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CandidateId(pub u32);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AqlPermissionPruning {
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

    pub(crate) fn try_aql_index_for_view(&self, view: &AgentView) -> EngineResult<EngineAqlIndex> {
        let analyzer = self.text_analyzer();
        let readable_scopes = crate::plan::PolicyRewrite::new(view)
            .readable_scopes()
            .collect::<BTreeSet<_>>();
        let mut index = if self.manifest().live_segments.is_empty() {
            EngineAqlIndex::try_from_delta_with_analyzer(&self.aql_delta_index, &analyzer)?
        } else {
            let (persisted, pruning) =
                self.persisted_index_state_for_readable_scopes(&readable_scopes)?;
            let mut index = EngineAqlIndex::from_persisted_delta_with_analyzer(
                persisted.bitmap,
                persisted.lexical,
                persisted.candidate_to_cell,
                &self.aql_delta_index,
                &analyzer,
            )?;
            index.permission_pruning = pruning;
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
