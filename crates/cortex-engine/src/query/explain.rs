use cortex_aql::{eval_bitmap_program, AgentView, BitmapProvider, BrainId, RetrievalMode};

use crate::context::ContextPackOptions;
use crate::database::{cell_version_meets_quality_thresholds, CandidateResolver, Database};
use crate::error::{EngineError, EngineResult};
use crate::exec::{PackOp, PhysicalOperatorTrace};
use crate::feedback::current_unix_seconds;
use crate::plan::{
    choose_retrieve_path, CostModelDecision, CostModelOptions, LogicalPlan, LogicalPlanReport,
    PolicyRewrite,
};

use super::cache::AqlStatementKind;
use super::EngineAqlProvider;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AqlExplainReport {
    pub task: String,
    pub brain_id: BrainId,
    pub selected_mode: RetrievalMode,
    pub logical_plan: LogicalPlanReport,
    pub policy_rewritten_plan: LogicalPlanReport,
    pub bitmap_plan: String,
    pub bitmap_ops: Vec<String>,
    pub filters: Vec<AqlExplainFilter>,
    pub cost_model: CostModelDecision,
    pub candidate_counts: AqlCandidateCounts,
    pub candidate_limit: u32,
    pub budget_tokens: u32,
    pub citations_required: bool,
    pub execution_trace: Option<AqlExecutionTraceReport>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AqlExplainFilter {
    pub kind: String,
    pub expression: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AqlCandidateCounts {
    pub universe: usize,
    pub agent_allowed: usize,
    pub live: usize,
    pub estimated_after_bitmap: Option<usize>,
    pub after_bitmap: usize,
    pub after_quality: usize,
    pub returned_limit: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AqlExecutionTraceReport {
    pub operators: Vec<PhysicalOperatorTrace>,
    pub total_elapsed_nanos: u64,
}

impl Database {
    pub fn explain_retrieve_aql(
        &self,
        aql: &str,
        view: &AgentView,
    ) -> EngineResult<AqlExplainReport> {
        let (cached, index) = self.bind_aql_cached(aql, view)?;
        if cached.statement_kind != AqlStatementKind::ExplainRetrieve {
            return Err(EngineError::InvalidOperation);
        }
        let cortex_aql::BoundPlan::Retrieve(plan) = cached.bound_plan else {
            return Err(EngineError::InvalidOperation);
        };
        self.explain_bound_retrieve(plan, cached.where_expression, index, view, false)
    }

    pub fn explain_analyze_retrieve_aql(
        &self,
        aql: &str,
        view: &AgentView,
    ) -> EngineResult<AqlExplainReport> {
        let (cached, index) = self.bind_aql_cached(aql, view)?;
        if cached.statement_kind != AqlStatementKind::ExplainAnalyzeRetrieve {
            return Err(EngineError::InvalidOperation);
        }
        let cortex_aql::BoundPlan::Retrieve(plan) = cached.bound_plan else {
            return Err(EngineError::InvalidOperation);
        };
        self.explain_bound_retrieve(plan, cached.where_expression, index, view, true)
    }

    fn explain_bound_retrieve(
        &self,
        plan: Box<cortex_aql::BoundRetrievePlan>,
        where_expression: Option<String>,
        index: crate::query::EngineAqlIndex,
        view: &AgentView,
        analyze: bool,
    ) -> EngineResult<AqlExplainReport> {
        let logical_plan = LogicalPlan::from_bound_plan(
            &cortex_aql::BoundPlan::Retrieve(plan.clone()),
            where_expression.as_deref(),
        );
        let policy_rewritten_plan = PolicyRewrite::new(view).rewrite(&logical_plan);
        let provider = EngineAqlProvider::new(index, view);
        let execution = if analyze {
            let mut execution = self.retrieve_cells_with_execution_trace(&plan, &provider)?;
            let feedback_scores = self.feedback_scores_at(current_unix_seconds());
            let budget = view.effective_budget(plan.context_policy.budget_tokens);
            let pack_execution = PackOp::execute(
                std::mem::take(&mut execution.cells),
                budget,
                plan.context_policy.require_citations,
                &ContextPackOptions::default(),
                &plan.task,
                &feedback_scores,
                Some(view),
            );
            execution.total_elapsed_nanos = execution
                .total_elapsed_nanos
                .saturating_add(pack_execution.trace.elapsed_nanos);
            execution.operators.push(pack_execution.trace);
            Some(execution)
        } else {
            None
        };
        let cost_model = execution
            .as_ref()
            .map(|execution| execution.cost_model.clone())
            .unwrap_or_else(|| {
                choose_retrieve_path(
                    &plan,
                    self.statistics(),
                    &provider,
                    &CostModelOptions::default(),
                )
            });
        let effective_candidate_limit = cost_model
            .recommended_candidate_limit
            .min(plan.context_policy.candidate_limit)
            as usize;
        let (after_bitmap, after_quality, returned_limit) = if let Some(execution) = &execution {
            (
                operator_output_count(&execution.operators, "BitmapIndexScan"),
                operator_output_count(&execution.operators, "QualityFilter"),
                operator_output_count(&execution.operators, "LimitOp"),
            )
        } else {
            let bitmap_candidates = eval_bitmap_program(&plan.bitmap_program, &provider)?;
            let pin = self.pin_read_txn();
            let txn = pin.read_txn();
            let after_quality = bitmap_candidates
                .iter()
                .filter_map(|candidate| provider.cell_id_for_candidate(*candidate))
                .filter_map(|cell_id| self.memtable.read(txn, cell_id))
                .filter(|version| {
                    cell_version_meets_quality_thresholds(version, &plan.quality_thresholds)
                })
                .count();
            (
                bitmap_candidates.len(),
                after_quality,
                after_quality.min(effective_candidate_limit),
            )
        };
        let estimated_after_bitmap = cost_model
            .estimated_after_bitmap
            .and_then(|rows| usize::try_from(rows).ok());
        let mut filters = vec![
            AqlExplainFilter {
                kind: "policy".to_owned(),
                expression: "agent_allowed".to_owned(),
            },
            AqlExplainFilter {
                kind: "liveness".to_owned(),
                expression: "live".to_owned(),
            },
        ];
        if let Some(expression) = where_expression {
            filters.push(AqlExplainFilter {
                kind: "where".to_owned(),
                expression,
            });
        }
        Ok(AqlExplainReport {
            task: plan.task.clone(),
            brain_id: plan.brain_id,
            selected_mode: plan.mode,
            logical_plan: logical_plan.to_report(),
            policy_rewritten_plan: policy_rewritten_plan.to_report(),
            bitmap_plan: plan.bitmap_program.explain(),
            bitmap_ops: plan
                .bitmap_program
                .ops
                .iter()
                .map(|op| format!("{op:?}"))
                .collect(),
            filters,
            cost_model,
            candidate_counts: AqlCandidateCounts {
                universe: provider.universe().len(),
                agent_allowed: provider.agent_allowed().len(),
                live: provider.live().len(),
                estimated_after_bitmap,
                after_bitmap,
                after_quality,
                returned_limit,
            },
            candidate_limit: plan.context_policy.candidate_limit,
            budget_tokens: plan.context_policy.budget_tokens,
            citations_required: plan.context_policy.require_citations,
            execution_trace: execution.map(|execution| AqlExecutionTraceReport {
                operators: execution.operators,
                total_elapsed_nanos: execution.total_elapsed_nanos,
            }),
        })
    }
}

fn operator_output_count(operators: &[PhysicalOperatorTrace], name: &str) -> usize {
    operators
        .iter()
        .find(|operator| operator.name == name)
        .map(|operator| operator.output_count)
        .unwrap_or_default()
}
