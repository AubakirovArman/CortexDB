use std::collections::BTreeSet;
use std::time::Instant;

use cortex_aql::{
    eval_bitmap_program, BitmapHandle, BitmapOp, BitmapProvider, BoundRetrievePlan, RetrievalMode,
    RoaringBitmap,
};

mod budget;
mod hybrid;
mod memory_lifecycle_filter;
mod temporal_filter;

use budget::apply_candidate_budget;
use hybrid::hybrid_candidates;
use memory_lifecycle_filter::apply_memory_lifecycle_filter;
use temporal_filter::apply_temporal_validity_filter;

use super::pack::ExplainCollector;
use super::scans::{BitmapIndexScan, PermissionFilter, QualityFilter};
use super::trace::{drain, elapsed_nanos, MaterializedOp, PhysicalOp, PhysicalOperatorTrace};
use crate::access_capture::MAX_CAPTURED_ACCESS_DENIALS;
use crate::database::{
    diversify_retrieved_cells, expand_parent_context, rank_retrieved_cells_with_window,
    suppress_duplicate_content, CandidateResolver, CapturedAccessDenialSet, Database,
    RetrievedCell,
};
use crate::error::EngineResult;
use crate::plan::{choose_retrieve_path, CostModelDecision, CostModelOptions, ExecutionPath};
use crate::search::analyze_search_query;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RetrieveExecutionReport {
    pub cells: Vec<RetrievedCell>,
    pub captured_access_denials: CapturedAccessDenialSet,
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

    let source = candidate_source(database, plan, provider, &cost_model, &mut collector)?;
    let mut captured_access_denials = source.captured_access_denials;

    let candidates = if source.permission_applied {
        source.candidates
    } else {
        let permission_input = source.candidates;
        let mut permission_filter = PermissionFilter::new(provider, permission_input.clone());
        let candidates = drain(&mut permission_filter);
        collector.push(permission_filter.trace());
        merge_access_denials(
            &mut captured_access_denials,
            capture_access_denials(provider, &permission_input, &candidates),
        );
        candidates
    };

    let candidates =
        apply_temporal_validity_filter(database, plan, provider, candidates, &mut collector);
    let candidates = apply_memory_lifecycle_filter(database, provider, candidates, &mut collector);

    let limit = cost_model
        .recommended_candidate_limit
        .min(plan.context_policy.candidate_limit) as usize;
    let candidates = apply_candidate_budget(plan, provider, candidates, limit, &mut collector);

    let mut quality_filter = QualityFilter::new(database, provider, plan, candidates);
    let cells = drain(&mut quality_filter);
    collector.push(quality_filter.trace());

    let started = Instant::now();
    let rank_input_count = cells.len();
    let ranked = rank_retrieved_cells_with_window(
        cells,
        &plan.task,
        &plan.weights,
        database.retrieval_recency_window_seconds,
    );
    let mut rank_op =
        MaterializedOp::new("RankOp", rank_input_count, ranked, elapsed_nanos(started));
    let ranked = drain(&mut rank_op);
    collector.push(rank_op.trace());

    // Optional MMR diversification (off by default; a no-op unless a Q16 lambda
    // < 65535 is configured on the database). Applied after ranking so the most
    // relevant cell stays first while near-duplicates are demoted.
    let ranked = if let Some(lambda) = database.retrieval_diversify_lambda_q16 {
        let started = Instant::now();
        let input_count = ranked.len();
        let diversified = diversify_retrieved_cells(ranked, lambda);
        let mut diversify_op = MaterializedOp::new(
            "DiversifyOp",
            input_count,
            diversified,
            elapsed_nanos(started),
        );
        let diversified = drain(&mut diversify_op);
        collector.push(diversify_op.trace());
        diversified
    } else {
        ranked
    };

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
        captured_access_denials,
        cost_model,
        operators: collector.into_traces(),
        total_elapsed_nanos: elapsed_nanos(started_total),
    })
}

struct CandidateSource {
    candidates: Vec<u32>,
    permission_applied: bool,
    captured_access_denials: CapturedAccessDenialSet,
}

pub(super) struct CandidateBatch {
    candidates: Vec<u32>,
    captured_access_denials: CapturedAccessDenialSet,
}

fn candidate_source<P: CandidateResolver>(
    database: &Database,
    plan: &BoundRetrievePlan,
    provider: &P,
    decision: &CostModelDecision,
    collector: &mut ExplainCollector,
) -> EngineResult<CandidateSource> {
    if plan.mode == RetrievalMode::Hybrid {
        return hybrid_candidates(database, plan, provider, collector);
    }
    if decision.selected_path == ExecutionPath::LexicalFirst {
        if let Some(source) = lexical_first_candidates(plan, provider, decision, collector)? {
            return Ok(source);
        }
    }
    let batch = bitmap_first_candidates(plan, provider, collector)?;
    Ok(CandidateSource {
        candidates: batch.candidates,
        permission_applied: false,
        captured_access_denials: batch.captured_access_denials,
    })
}

pub(super) fn capture_access_denials<P: CandidateResolver>(
    provider: &P,
    input_candidates: &[u32],
    output_candidates: &[u32],
) -> CapturedAccessDenialSet {
    let output = output_candidates.iter().copied().collect::<BTreeSet<_>>();
    let mut total_denied = 0;
    let mut denials = Vec::new();
    for candidate in input_candidates {
        if output.contains(candidate) {
            continue;
        }
        total_denied += 1;
        if denials.len() >= MAX_CAPTURED_ACCESS_DENIALS {
            continue;
        }
        if let Some(denial) = provider.captured_access_denial_for_candidate(*candidate) {
            denials.push(denial);
        }
    }
    CapturedAccessDenialSet {
        total_denied,
        truncated: total_denied > denials.len(),
        denials,
    }
}

fn capture_agent_allowed_bitmap_denials<P: CandidateResolver>(
    plan: &BoundRetrievePlan,
    provider: &P,
    output_candidates: &[u32],
) -> EngineResult<CapturedAccessDenialSet> {
    if !plan
        .bitmap_program
        .ops
        .iter()
        .any(|op| matches!(op, BitmapOp::PushAgentAllowed))
    {
        return Ok(CapturedAccessDenialSet::default());
    }
    let bypass = AgentAllowedBypassProvider { provider };
    let input_candidates = eval_bitmap_program(&plan.bitmap_program, &bypass)?
        .into_iter()
        .collect::<Vec<_>>();
    Ok(capture_access_denials(
        provider,
        &input_candidates,
        output_candidates,
    ))
}

pub(super) fn merge_access_denials(
    target: &mut CapturedAccessDenialSet,
    additional: CapturedAccessDenialSet,
) {
    if additional.total_denied == 0 {
        return;
    }
    target.total_denied += additional.total_denied;
    target.truncated |= additional.truncated;
    let mut existing = target
        .denials
        .iter()
        .map(|denial| denial.candidate)
        .collect::<BTreeSet<_>>();
    for denial in additional.denials {
        if !existing.insert(denial.candidate) {
            continue;
        }
        if target.denials.len() >= MAX_CAPTURED_ACCESS_DENIALS {
            target.truncated = true;
            continue;
        }
        target.denials.push(denial);
    }
    if target.denials.len() < target.total_denied {
        target.truncated = true;
    }
}

struct AgentAllowedBypassProvider<'a, P> {
    provider: &'a P,
}

impl<P: BitmapProvider> BitmapProvider for AgentAllowedBypassProvider<'_, P> {
    fn bitmap(&self, handle: BitmapHandle) -> Option<RoaringBitmap> {
        self.provider.bitmap(handle)
    }

    fn agent_allowed(&self) -> RoaringBitmap {
        self.provider.universe()
    }

    fn live(&self) -> RoaringBitmap {
        self.provider.live()
    }

    fn universe(&self) -> RoaringBitmap {
        self.provider.universe()
    }
}

fn bitmap_first_candidates<P: CandidateResolver>(
    plan: &BoundRetrievePlan,
    provider: &P,
    collector: &mut ExplainCollector,
) -> EngineResult<CandidateBatch> {
    let mut scan = BitmapIndexScan::from_plan(plan, provider)?;
    let candidates = drain(&mut scan);
    collector.push(scan.trace());
    let captured_access_denials =
        capture_agent_allowed_bitmap_denials(plan, provider, &candidates)?;
    Ok(CandidateBatch {
        candidates,
        captured_access_denials,
    })
}

fn lexical_first_candidates<P: CandidateResolver>(
    plan: &BoundRetrievePlan,
    provider: &P,
    decision: &CostModelDecision,
    collector: &mut ExplainCollector,
) -> EngineResult<Option<CandidateSource>> {
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
    let captured_access_denials =
        capture_agent_allowed_bitmap_denials(plan, provider, &bitmap_candidates)?;

    let lexical_set = lexical_candidates.into_iter().collect::<BTreeSet<_>>();
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
    Ok(Some(CandidateSource {
        candidates,
        permission_applied: false,
        captured_access_denials,
    }))
}
