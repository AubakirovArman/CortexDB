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
use crate::plan::{choose_retrieve_path, CostModelDecision, CostModelOptions, ExecutionPath};
use crate::search::analyze_search_query;

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

    let candidates = candidate_source(plan, provider, &cost_model, &mut collector)?;

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

    let limit = cost_model
        .recommended_candidate_limit
        .min(plan.context_policy.candidate_limit) as usize;
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

fn candidate_source<P: CandidateResolver>(
    plan: &BoundRetrievePlan,
    provider: &P,
    decision: &CostModelDecision,
    collector: &mut ExplainCollector,
) -> EngineResult<Vec<u32>> {
    if decision.selected_path == ExecutionPath::LexicalFirst {
        if let Some(candidates) = lexical_first_candidates(plan, provider, decision, collector)? {
            return Ok(candidates);
        }
    }
    bitmap_first_candidates(plan, provider, collector)
}

fn bitmap_first_candidates<P: CandidateResolver>(
    plan: &BoundRetrievePlan,
    provider: &P,
    collector: &mut ExplainCollector,
) -> EngineResult<Vec<u32>> {
    let mut scan = BitmapIndexScan::from_plan(plan, provider)?;
    let candidates = drain(&mut scan);
    collector.push(scan.trace());
    Ok(candidates)
}

fn lexical_first_candidates<P: CandidateResolver>(
    plan: &BoundRetrievePlan,
    provider: &P,
    decision: &CostModelDecision,
    collector: &mut ExplainCollector,
) -> EngineResult<Option<Vec<u32>>> {
    let terms = if let Some(term) = &decision.rarest_term {
        vec![term.term.clone()]
    } else {
        analyze_search_query(&plan.task)
            .weighted_terms
            .keys()
            .cloned()
            .collect::<Vec<_>>()
    };
    let Some(lexical_candidates) = provider.lexical_candidates_for_terms(&terms) else {
        return Ok(None);
    };
    if lexical_candidates.is_empty() {
        return Ok(None);
    }

    let mut source_traces = Vec::new();
    let started = Instant::now();
    let mut lexical_scan = MaterializedOp::new(
        "LexicalScan",
        terms.len(),
        lexical_candidates.iter().copied().collect::<Vec<_>>(),
        elapsed_nanos(started),
    );
    let lexical_candidates = drain(&mut lexical_scan);
    source_traces.push(lexical_scan.trace());

    let mut bitmap_scan = BitmapIndexScan::from_plan(plan, provider)?;
    let bitmap_candidates = drain(&mut bitmap_scan);
    source_traces.push(bitmap_scan.trace());

    let lexical_set = lexical_candidates
        .into_iter()
        .collect::<std::collections::BTreeSet<_>>();
    let started = Instant::now();
    let intersected = bitmap_candidates
        .iter()
        .copied()
        .filter(|candidate| lexical_set.contains(candidate))
        .collect::<Vec<_>>();
    if intersected.is_empty() {
        return Ok(None);
    }
    let mut intersect_op = MaterializedOp::new(
        "BitmapIntersectOp",
        bitmap_candidates.len(),
        intersected,
        elapsed_nanos(started),
    );
    let candidates = drain(&mut intersect_op);
    source_traces.push(intersect_op.trace());
    for trace in source_traces {
        collector.push(trace);
    }
    Ok(Some(candidates))
}
