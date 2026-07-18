use cortex_aql::BoundRetrievePlan;

use crate::database::{CandidateResolver, Database};
use crate::exec::pack::ExplainCollector;
use crate::exec::trace::{drain, elapsed_nanos, MaterializedOp, PhysicalOp};

pub(super) fn apply_temporal_validity_filter<P: CandidateResolver>(
    database: &Database,
    plan: &BoundRetrievePlan,
    provider: &P,
    candidates: Vec<u32>,
    collector: &mut ExplainCollector,
) -> Vec<u32> {
    let Some(valid_at) = plan.quality_thresholds.valid_at.as_deref() else {
        return candidates;
    };
    let input_count = candidates.len();
    let started = std::time::Instant::now();
    let valid_cell_ids = database
        .derived_stores
        .temporal_validity_store
        .valid_cell_ids_at(valid_at);
    let filtered = candidates
        .into_iter()
        .filter(|candidate| {
            provider
                .cell_id_for_candidate(*candidate)
                .map(|cell_id| valid_cell_ids.contains(&cell_id))
                .unwrap_or(false)
        })
        .collect::<Vec<_>>();
    let mut op = MaterializedOp::new(
        "TemporalValidityFilter",
        input_count,
        filtered,
        elapsed_nanos(started),
    );
    let candidates = drain(&mut op);
    collector.push(op.trace());
    candidates
}
