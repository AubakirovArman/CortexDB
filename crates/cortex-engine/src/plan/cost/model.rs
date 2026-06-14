use cortex_aql::{BitmapOp, BitmapProgram, BitmapProvider, BoundRetrievePlan, RetrievalMode};

use super::{
    CostModelDecision, CostModelEstimate, CostModelOptions, ExecutionPath, TermDfEstimate,
    VectorSearchDecision, VectorSearchExecution,
};
use crate::query::DatabaseStatistics;
use crate::search::analyze_search_query;

pub const ANN_VECTOR_MIN_LIVE_ROWS: u64 = 1_000_000;

pub fn choose_retrieve_path<P: BitmapProvider>(
    plan: &BoundRetrievePlan,
    statistics: DatabaseStatistics<'_>,
    provider: &P,
    options: &CostModelOptions,
) -> CostModelDecision {
    let live_rows = statistics.live_segment_row_count();
    let estimated_after_bitmap =
        estimate_bitmap_program_rows(&plan.bitmap_program, statistics, provider);
    let has_query_vector = task_has_query_vector(&plan.task);
    let rarest_term = rarest_term_estimate(&plan.task, statistics);
    let recommended_candidate_limit = recommended_candidate_limit(
        plan.context_policy.candidate_limit,
        plan.context_policy.budget_tokens,
    );
    let estimates = cost_estimates(
        plan,
        live_rows,
        estimated_after_bitmap,
        rarest_term.as_ref(),
        has_query_vector,
        recommended_candidate_limit,
    );

    if let Some(forced_path) = options.forced_path {
        return CostModelDecision {
            selected_path: forced_path,
            reason: "forced by debug override".to_owned(),
            estimated_live_rows: live_rows,
            estimated_after_bitmap,
            recommended_candidate_limit,
            has_query_vector,
            rarest_term,
            estimates,
        };
    }

    let candidate_rows = estimated_after_bitmap.unwrap_or(live_rows);
    if plan.mode == RetrievalMode::Hybrid {
        return decision(
            ExecutionPath::Hybrid,
            "hybrid mode fuses lexical and vector evidence",
            DecisionContext {
                live_rows,
                estimated_after_bitmap,
                recommended_candidate_limit,
                has_query_vector,
                rarest_term,
                estimates,
            },
        );
    }

    if is_narrow_bitmap(candidate_rows, live_rows) {
        return decision(
            ExecutionPath::BitmapFirst,
            "bitmap predicate is selective",
            DecisionContext {
                live_rows,
                estimated_after_bitmap,
                recommended_candidate_limit,
                has_query_vector,
                rarest_term,
                estimates,
            },
        );
    }

    if let Some(term) = &rarest_term {
        if is_rare_term(term.document_frequency, live_rows) {
            return decision(
                ExecutionPath::LexicalFirst,
                "rare term df is selective",
                DecisionContext {
                    live_rows,
                    estimated_after_bitmap,
                    recommended_candidate_limit,
                    has_query_vector,
                    rarest_term,
                    estimates,
                },
            );
        }
    }

    if has_query_vector && vector_should_lead(plan.mode, candidate_rows, live_rows) {
        return decision(
            ExecutionPath::VectorFirst,
            "semantic vector is available and bitmap is broad",
            DecisionContext {
                live_rows,
                estimated_after_bitmap,
                recommended_candidate_limit,
                has_query_vector,
                rarest_term,
                estimates,
            },
        );
    }

    if plan.mode == RetrievalMode::Balanced || plan.mode == RetrievalMode::Semantic {
        return decision(
            ExecutionPath::Hybrid,
            "balanced retrieval keeps lexical and semantic evidence",
            DecisionContext {
                live_rows,
                estimated_after_bitmap,
                recommended_candidate_limit,
                has_query_vector,
                rarest_term,
                estimates,
            },
        );
    }

    decision(
        ExecutionPath::BitmapFirst,
        "default safe bitmap-first path",
        DecisionContext {
            live_rows,
            estimated_after_bitmap,
            recommended_candidate_limit,
            has_query_vector,
            rarest_term,
            estimates,
        },
    )
}

pub fn choose_vector_search_execution(
    statistics: DatabaseStatistics<'_>,
    estimated_candidate_rows: Option<u64>,
    hnsw_enabled: bool,
) -> VectorSearchDecision {
    let live_rows = statistics.live_segment_row_count();
    let candidate_rows = estimated_candidate_rows.unwrap_or(live_rows);
    if !hnsw_enabled {
        return vector_search_decision(
            VectorSearchExecution::Exact,
            "hnsw is disabled",
            live_rows,
            candidate_rows,
            hnsw_enabled,
        );
    }
    if live_rows < ANN_VECTOR_MIN_LIVE_ROWS {
        return vector_search_decision(
            VectorSearchExecution::Exact,
            "corpus is below ann threshold",
            live_rows,
            candidate_rows,
            hnsw_enabled,
        );
    }
    if is_narrow_bitmap(candidate_rows, live_rows) {
        return vector_search_decision(
            VectorSearchExecution::Exact,
            "candidate predicate is selective",
            live_rows,
            candidate_rows,
            hnsw_enabled,
        );
    }
    vector_search_decision(
        VectorSearchExecution::AnnWithExactFallback,
        "large corpus uses ann guarded by exact fallback",
        live_rows,
        candidate_rows,
        hnsw_enabled,
    )
}

pub fn estimate_bitmap_program_rows<P: BitmapProvider>(
    program: &BitmapProgram,
    statistics: DatabaseStatistics<'_>,
    provider: &P,
) -> Option<u64> {
    let max_rows = statistics.live_segment_row_count();
    let mut stack = Vec::<Option<u64>>::new();
    for op in &program.ops {
        match op {
            BitmapOp::Push(handle) => stack.push(statistics.estimate_bitmap_cardinality(*handle)),
            BitmapOp::PushUniverse | BitmapOp::PushLive => stack.push(Some(max_rows)),
            BitmapOp::PushAgentAllowed => stack.push(Some(provider.agent_allowed().len())),
            BitmapOp::And => {
                let rhs = stack.pop()??;
                let lhs = stack.pop()??;
                stack.push(Some(lhs.min(rhs)));
            }
            BitmapOp::Or => {
                let rhs = stack.pop()??;
                let lhs = stack.pop()??;
                stack.push(Some(lhs.saturating_add(rhs).min(max_rows)));
            }
            BitmapOp::Not => {
                let value = stack.pop()??;
                stack.push(Some(max_rows.saturating_sub(value)));
            }
        }
    }
    let [Some(rows)] = stack.as_slice() else {
        return None;
    };
    Some(*rows)
}

fn vector_search_decision(
    execution: VectorSearchExecution,
    reason: &str,
    live_rows: u64,
    candidate_rows: u64,
    hnsw_enabled: bool,
) -> VectorSearchDecision {
    VectorSearchDecision {
        execution,
        reason: reason.to_owned(),
        estimated_live_rows: live_rows,
        estimated_candidate_rows: candidate_rows,
        hnsw_enabled,
    }
}

struct DecisionContext {
    live_rows: u64,
    estimated_after_bitmap: Option<u64>,
    recommended_candidate_limit: u32,
    has_query_vector: bool,
    rarest_term: Option<TermDfEstimate>,
    estimates: Vec<CostModelEstimate>,
}

fn decision(
    selected_path: ExecutionPath,
    reason: &str,
    context: DecisionContext,
) -> CostModelDecision {
    CostModelDecision {
        selected_path,
        reason: reason.to_owned(),
        estimated_live_rows: context.live_rows,
        estimated_after_bitmap: context.estimated_after_bitmap,
        recommended_candidate_limit: context.recommended_candidate_limit,
        has_query_vector: context.has_query_vector,
        rarest_term: context.rarest_term,
        estimates: context.estimates,
    }
}

fn rarest_term_estimate(task: &str, statistics: DatabaseStatistics<'_>) -> Option<TermDfEstimate> {
    analyze_search_query(&task_without_vector_lines(task))
        .weighted_terms
        .keys()
        .filter_map(|term| {
            statistics
                .estimate_term_document_frequency(term)
                .map(|document_frequency| TermDfEstimate {
                    term: term.clone(),
                    document_frequency,
                })
        })
        .filter(|estimate| estimate.document_frequency > 0)
        .min_by(|left, right| {
            left.document_frequency
                .cmp(&right.document_frequency)
                .then_with(|| left.term.cmp(&right.term))
        })
}

fn task_has_query_vector(task: &str) -> bool {
    task.lines().any(|line| {
        let trimmed = line.trim();
        trimmed.strip_prefix("query_vector=").is_some() || trimmed.strip_prefix("vector=").is_some()
    })
}

fn task_without_vector_lines(task: &str) -> String {
    task.lines()
        .filter(|line| {
            let trimmed = line.trim();
            !trimmed.starts_with("query_vector=") && !trimmed.starts_with("vector=")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn is_narrow_bitmap(candidate_rows: u64, live_rows: u64) -> bool {
    if live_rows == 0 {
        return true;
    }
    candidate_rows <= 16 || candidate_rows.saturating_mul(20) <= live_rows
}

fn is_rare_term(document_frequency: u64, live_rows: u64) -> bool {
    if live_rows == 0 {
        return false;
    }
    let threshold = (live_rows / 50).max(4);
    document_frequency <= threshold
}

fn vector_should_lead(mode: RetrievalMode, candidate_rows: u64, live_rows: u64) -> bool {
    if mode != RetrievalMode::Semantic {
        return false;
    }
    if live_rows == 0 {
        return true;
    }
    candidate_rows.saturating_mul(4) >= live_rows
}

fn recommended_candidate_limit(candidate_limit: u32, budget_tokens: u32) -> u32 {
    let budget_cap = (budget_tokens / 160).max(1);
    candidate_limit.min(budget_cap)
}

fn cost_estimates(
    plan: &BoundRetrievePlan,
    live_rows: u64,
    estimated_after_bitmap: Option<u64>,
    rarest_term: Option<&TermDfEstimate>,
    has_query_vector: bool,
    recommended_candidate_limit: u32,
) -> Vec<CostModelEstimate> {
    let candidate_rows = estimated_after_bitmap.unwrap_or(live_rows).max(1);
    let lexical_rows = rarest_term
        .map(|term| term.document_frequency.max(1))
        .unwrap_or(candidate_rows);
    let vector_rows = if has_query_vector {
        candidate_rows.saturating_mul(4)
    } else {
        u64::MAX / 4
    };
    let bitmap_cost = candidate_rows.saturating_add(plan.bitmap_program.estimated_cost());
    let lexical_cost = lexical_rows.saturating_add(plan.bitmap_program.estimated_cost());
    let hybrid_cost = lexical_cost.saturating_add(vector_rows / 2);
    let pack_cost = u64::from(recommended_candidate_limit).saturating_mul(160);

    vec![
        CostModelEstimate {
            path: ExecutionPath::BitmapFirst,
            cost_units: bitmap_cost,
        },
        CostModelEstimate {
            path: ExecutionPath::LexicalFirst,
            cost_units: lexical_cost,
        },
        CostModelEstimate {
            path: ExecutionPath::VectorFirst,
            cost_units: vector_rows,
        },
        CostModelEstimate {
            path: ExecutionPath::Hybrid,
            cost_units: hybrid_cost,
        },
        CostModelEstimate {
            path: ExecutionPath::Pack,
            cost_units: pack_cost,
        },
    ]
}
