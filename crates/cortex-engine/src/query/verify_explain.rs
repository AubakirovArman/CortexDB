use cortex_aql::{AgentView, BoundPlan, BrainId};

use super::cache::AqlStatementKind;
use super::explain::AqlExecutionTraceReport;
use crate::database::Database;
use crate::error::{EngineError, EngineResult};
use crate::plan::{LogicalPlan, LogicalPlanReport, PolicyRewrite};
use crate::verification::VerificationStatus;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AqlVerifyExplainReport {
    pub fact: String,
    pub brain_id: BrainId,
    pub max_candidates: u32,
    pub max_evidence: u32,
    pub logical_plan: LogicalPlanReport,
    pub policy_rewritten_plan: LogicalPlanReport,
    pub execution_trace: Option<AqlExecutionTraceReport>,
    pub status: Option<VerificationStatus>,
    pub confidence_q16: Option<u16>,
    pub evidence_count: usize,
    pub contradicting_evidence_count: usize,
    pub guard_count: usize,
    pub numeric_conflict_count: usize,
}

impl Database {
    pub fn explain_verify_aql(
        &self,
        aql: &str,
        view: &AgentView,
    ) -> EngineResult<AqlVerifyExplainReport> {
        self.explain_bound_verify(aql, view, false)
    }

    pub fn explain_analyze_verify_aql(
        &self,
        aql: &str,
        view: &AgentView,
    ) -> EngineResult<AqlVerifyExplainReport> {
        self.explain_bound_verify(aql, view, true)
    }

    fn explain_bound_verify(
        &self,
        aql: &str,
        view: &AgentView,
        analyze: bool,
    ) -> EngineResult<AqlVerifyExplainReport> {
        let (cached, _) = self.bind_aql_cached(aql, view)?;
        if cached.statement_kind != AqlStatementKind::ExplainOther {
            return Err(EngineError::InvalidOperation);
        }
        let BoundPlan::VerifyFact(plan) = cached.bound_plan else {
            return Err(EngineError::InvalidOperation);
        };
        let plan = *plan;
        let bound_plan = BoundPlan::VerifyFact(Box::new(plan.clone()));
        let logical_plan = LogicalPlan::from_bound_plan(&bound_plan, None);
        let policy_rewritten_plan = PolicyRewrite::new(view).rewrite(&logical_plan);
        let execution = if analyze {
            Some(self.execute_verify_fact_plan(plan.clone(), view)?)
        } else {
            None
        };
        let (
            execution_trace,
            status,
            confidence_q16,
            evidence_count,
            contradiction_count,
            guard_count,
            numeric_conflict_count,
        ) = if let Some(execution) = execution {
            let report = execution.report;
            (
                Some(AqlExecutionTraceReport {
                    operators: execution.operators,
                    total_elapsed_nanos: execution.total_elapsed_nanos,
                }),
                Some(report.status),
                Some(report.confidence_q16),
                report.evidence.len(),
                report.contradicting_evidence.len(),
                report.guards.len(),
                report.numeric_conflicts.len(),
            )
        } else {
            (None, None, None, 0, 0, 0, 0)
        };
        Ok(AqlVerifyExplainReport {
            fact: plan.fact,
            brain_id: plan.brain_id,
            max_candidates: plan.max_candidates,
            max_evidence: plan.max_evidence,
            logical_plan: logical_plan.to_report(),
            policy_rewritten_plan: policy_rewritten_plan.to_report(),
            execution_trace,
            status,
            confidence_q16,
            evidence_count,
            contradicting_evidence_count: contradiction_count,
            guard_count,
            numeric_conflict_count,
        })
    }
}
