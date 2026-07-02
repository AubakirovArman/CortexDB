use std::collections::BTreeSet;

use cortex_aql::{eval_bitmap_program, AgentView, BoundPlan, BoundRetrievePlan};
use cortex_core::CellId;

use crate::checkpoint::PersistedIndexState;
use crate::database::{CandidateResolver, Database};
use crate::error::EngineResult;
use crate::query::EngineAqlProvider;

use super::super::access::allowed_candidates;

pub(super) fn persisted_allowed_candidates(
    state: &PersistedIndexState,
    view: &AgentView,
    allowed_cells: Option<&BTreeSet<CellId>>,
) -> BTreeSet<u32> {
    let mut allowed = allowed_candidates(&state.bitmap, view);
    if let Some(allowed_cells) = allowed_cells {
        allowed.retain(|candidate| {
            state
                .candidate_to_cell
                .get(candidate)
                .is_some_and(|cell_id| allowed_cells.contains(cell_id))
        });
    }
    allowed
}

pub(super) fn allowed_cells_from_bound_retrieve_plan(
    database: &Database,
    view: &AgentView,
    plan: &BoundRetrievePlan,
) -> EngineResult<BTreeSet<CellId>> {
    let bound_plan = BoundPlan::Retrieve(Box::new(plan.clone()));
    let index = database.try_aql_index_for_bound_plan(view, &bound_plan)?;
    let provider = EngineAqlProvider::new(index, view);
    let candidates = eval_bitmap_program(&plan.bitmap_program, &provider)?;
    Ok(candidates
        .into_iter()
        .filter_map(|candidate| provider.cell_id_for_candidate(candidate))
        .collect())
}
