use std::collections::{BTreeMap, BTreeSet};

pub(crate) mod cache;
mod candidates;
mod catalog;
mod explain;
mod index;
pub(crate) mod metadata;
mod metadata_validation;
mod provider;
mod render;

use cortex_aql::{AgentView, BitmapHandle, BoundPlan, BrainId};
use cortex_core::{CellId, CommitSeq};

use crate::database::{Database, RetrievedCell};
use crate::error::{EngineError, EngineResult};
pub use cache::AqlQueryCacheStats;
pub use explain::{
    AqlCandidateCounts, AqlExecutionTraceReport, AqlExplainFilter, AqlExplainReport,
};
pub use metadata::{scope_id, CellMetadata, SourceRef};
pub use provider::EngineAqlProvider;

const DEFAULT_BRAIN: BrainId = BrainId(1);

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
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CandidateId(pub u32);

impl Database {
    pub fn aql_index(&self) -> EngineResult<EngineAqlIndex> {
        self.try_aql_index()
    }

    pub fn try_aql_index(&self) -> EngineResult<EngineAqlIndex> {
        let checkpoint_seq = CommitSeq(self.manifest().checkpoint_seq);
        let changed = self.memtable.changed_cell_ids_after(checkpoint_seq);
        if self.manifest().live_segments.is_empty() {
            return EngineAqlIndex::try_from_versions(&self.snapshot_versions());
        }
        let persisted = self.persisted_index_state()?;
        EngineAqlIndex::from_persisted(
            persisted.bitmap,
            persisted.lexical,
            persisted.candidate_to_cell,
            &self.snapshot_versions(),
            &changed,
        )
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
