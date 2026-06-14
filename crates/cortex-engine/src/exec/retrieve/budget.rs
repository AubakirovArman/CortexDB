use cortex_aql::BoundRetrievePlan;

use crate::database::CandidateResolver;
use crate::exec::pack::ExplainCollector;
use crate::exec::trace::{drain, elapsed_nanos, MaterializedOp, PhysicalOp};

pub(super) fn apply_candidate_budget<P: CandidateResolver>(
    plan: &BoundRetrievePlan,
    provider: &P,
    candidates: Vec<u32>,
    limit: usize,
    collector: &mut ExplainCollector,
) -> Vec<u32> {
    if candidates.len() <= limit || limit == 0 {
        return candidates;
    }
    let started = std::time::Instant::now();
    let Some(mut ranked) = provider.ranked_candidates_for_task(&plan.task, &candidates) else {
        return candidates;
    };
    let input_count = ranked.len();
    ranked.truncate(payload_fetch_limit(limit, input_count));
    let mut op = MaterializedOp::new(
        "CheapRankBudgetOp",
        input_count,
        ranked,
        elapsed_nanos(started),
    );
    let candidates = drain(&mut op);
    collector.push(op.trace());
    candidates
}

fn payload_fetch_limit(limit: usize, available: usize) -> usize {
    limit
        .saturating_mul(2)
        .max(limit.saturating_add(2))
        .min(available)
}
