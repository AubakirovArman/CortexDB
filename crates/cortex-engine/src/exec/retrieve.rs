use std::time::Instant;

use cortex_aql::BoundRetrievePlan;

use super::pack::ExplainCollector;
use super::scans::{BitmapIndexScan, PermissionFilter, QualityFilter};
use super::trace::{drain, elapsed_nanos, MaterializedOp, PhysicalOp, PhysicalOperatorTrace};
use crate::database::{
    expand_parent_context, rank_retrieved_cells, suppress_duplicate_content, CandidateResolver,
    Database, RetrievedCell,
};
use crate::error::EngineResult;
use crate::plan::{choose_retrieve_path, CostModelDecision, CostModelOptions};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RetrieveExecutionReport {
    pub cells: Vec<RetrievedCell>,
    pub cost_model: CostModelDecision,
    pub operators: Vec<PhysicalOperatorTrace>,
    pub total_elapsed_nanos: u64,
}

pub fn execute_retrieve<P: CandidateResolver>(
    database: &Database,
    plan: &BoundRetrievePlan,
    provider: &P,
) -> EngineResult<RetrieveExecutionReport> {
    let started_total = Instant::now();
    let cost_model = choose_retrieve_path(
        plan,
        database.statistics(),
        provider,
        &CostModelOptions::default(),
    );
    let mut collector = ExplainCollector::default();

    let mut scan = BitmapIndexScan::from_plan(plan, provider)?;
    let candidates = drain(&mut scan);
    collector.push(scan.trace());

    let mut permission_filter = PermissionFilter::new(provider, candidates);
    let candidates = drain(&mut permission_filter);
    collector.push(permission_filter.trace());

    let mut quality_filter = QualityFilter::new(database, provider, plan, candidates);
    let cells = drain(&mut quality_filter);
    collector.push(quality_filter.trace());

    let started = Instant::now();
    let rank_input_count = cells.len();
    let ranked = rank_retrieved_cells(cells, &plan.task, &plan.weights);
    let mut rank_op =
        MaterializedOp::new("RankOp", rank_input_count, ranked, elapsed_nanos(started));
    let ranked = drain(&mut rank_op);
    collector.push(rank_op.trace());

    let started = Instant::now();
    let deduped = suppress_duplicate_content(ranked);
    let mut dedup_op = MaterializedOp::new(
        "DedupOp",
        collector.last_output_count(),
        deduped,
        elapsed_nanos(started),
    );
    let deduped = drain(&mut dedup_op);
    collector.push(dedup_op.trace());

    let started = Instant::now();
    let expanded = expand_parent_context(deduped);
    let mut parent_op = MaterializedOp::new(
        "ParentExpandOp",
        collector.last_output_count(),
        expanded,
        elapsed_nanos(started),
    );
    let expanded = drain(&mut parent_op);
    collector.push(parent_op.trace());

    let limit = plan.context_policy.candidate_limit as usize;
    let started = Instant::now();
    let limited = expanded.into_iter().take(limit).collect::<Vec<_>>();
    let mut limit_op = MaterializedOp::new(
        "LimitOp",
        collector.last_output_count(),
        limited,
        elapsed_nanos(started),
    );
    let cells = drain(&mut limit_op);
    collector.push(limit_op.trace());

    Ok(RetrieveExecutionReport {
        cells,
        cost_model,
        operators: collector.into_traces(),
        total_elapsed_nanos: elapsed_nanos(started_total),
    })
}
