use crate::agent_view::AgentView;
use crate::ast::{AqlStatement, RawRemember, RawRetrieveContext, RawVerifyFact, TtlValue};
use crate::policy::PolicyValidator;
use crate::types::{
    BrainId, CellTypeId, MemoryType, RetrievalMode, ScopeId, StatusId, Q16, Q16_ZERO,
};
mod catalog;
mod diagnostics;
mod errors;
mod plan;
mod support;
mod where_bitmap;

pub use catalog::{BrainCatalog, CellTypeCatalog, MemoryTypeCatalog, ScopeCatalog, StatusCatalog};
pub use diagnostics::{BindDiagnostic, BindDiagnosticExport};
pub use errors::BindError;
pub use plan::{
    BoundPlan, BoundRememberPlan, BoundRetrievePlan, BoundVerifyFactPlan, ContextPolicy,
    QualityThresholds,
};
use support::apply_requirement;
pub use support::{
    compute_bitmap_stack_depth, context_policy_for_mode, decimal_to_q16, default_weights,
    normalize_weights, optimize_bitmap_ops,
};
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BitmapHandle(pub u64);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BitmapProgram {
    pub ops: Vec<BitmapOp>,
    pub max_stack_depth: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BitmapOp {
    Push(BitmapHandle),
    PushUniverse,
    PushAgentAllowed,
    PushLive,
    And,
    Or,
    /// Complement is evaluated only inside the segment-local universe.
    Not,
}

pub trait AqlCatalog {
    fn resolve_brain(&self, name: &str) -> Option<BrainId>;
    fn resolve_scope(&self, brain: BrainId, name: &str) -> Option<ScopeId>;
    fn resolve_write_scope(&self, _name: &str) -> Option<ScopeId> {
        None
    }
    fn resolve_status(&self, brain: BrainId, status: &str) -> Option<StatusId>;
    fn resolve_cell_type(&self, brain: BrainId, cell_type: &str) -> Option<CellTypeId>;
    fn scope_bitmap(&self, brain: BrainId, scope: ScopeId) -> Option<BitmapHandle>;
    fn status_bitmap(&self, brain: BrainId, status: StatusId) -> Option<BitmapHandle>;
    fn cell_type_bitmap(&self, brain: BrainId, cell_type: CellTypeId) -> Option<BitmapHandle>;
    fn memory_type_bitmap(&self, brain: BrainId, memory_type: MemoryType) -> Option<BitmapHandle>;
    fn field_is_filterable(&self, brain: BrainId, field: &str) -> bool;

    fn bitmap_estimated_cardinality(&self, _brain: BrainId, _handle: BitmapHandle) -> Option<u64> {
        None
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RetrievalWeights {
    pub lexical_q16: Q16,
    pub semantic_q16: Q16,
    pub recency_q16: Q16,
    pub trust_q16: Q16,
}

pub struct Binder<'a, C> {
    catalog: &'a C,
    view: &'a AgentView,
}

impl<'a, C: AqlCatalog> Binder<'a, C> {
    pub fn new(catalog: &'a C, view: &'a AgentView) -> Self {
        Self { catalog, view }
    }

    pub fn bind_statement(&self, statement: &AqlStatement<'_>) -> Result<BoundPlan, BindError> {
        match statement {
            AqlStatement::RetrieveContext(raw) => self
                .bind_retrieve(raw)
                .map(|plan| BoundPlan::Retrieve(Box::new(plan))),
            AqlStatement::VerifyFact(raw) => self
                .bind_verify_fact(raw)
                .map(|plan| BoundPlan::VerifyFact(Box::new(plan))),
            AqlStatement::Remember(raw) => self
                .bind_remember(raw)
                .map(|plan| BoundPlan::Remember(Box::new(plan))),
            AqlStatement::Explain(inner) | AqlStatement::ExplainAnalyze(inner) => {
                self.bind_statement(inner)
            }
        }
    }

    pub fn bind_retrieve(
        &self,
        raw: &RawRetrieveContext<'_>,
    ) -> Result<BoundRetrievePlan, BindError> {
        let brain = self
            .catalog
            .resolve_brain(raw.brain.node.value.as_ref())
            .ok_or(BindError::UnknownBrain)?;
        let mode = raw
            .mode
            .as_ref()
            .map_or(RetrievalMode::Balanced, |mode| mode.node);
        let budget_tokens = raw.budget_tokens.as_ref().map_or(
            u64::from(self.view.default_context_budget_tokens),
            |budget| budget.node,
        );
        let requested_budget = u32::try_from(budget_tokens).unwrap_or(u32::MAX);
        let requested_limit = raw
            .candidate_limit
            .as_ref()
            .map_or(self.view.default_candidate_limit, |limit| limit.node);
        let effective = PolicyValidator::new(self.view)
            .enforce_retrieve(brain, mode, requested_budget, requested_limit)
            .map_err(BindError::PolicyDenied)?;

        let mut thresholds = QualityThresholds {
            min_confidence_q16: effective.min_required_confidence_q16,
            min_source_trust_q16: Q16_ZERO,
            max_freshness_seconds: None,
            valid_at: None,
        };
        let mut context_policy = context_policy_for_mode(mode, self.view, &effective);
        for requirement in &raw.requirements {
            apply_requirement(&mut thresholds, &mut context_policy, &requirement.node)?;
        }

        let mut ops = vec![
            BitmapOp::PushAgentAllowed,
            BitmapOp::PushLive,
            BitmapOp::And,
        ];
        if let Some(condition) = &raw.where_clause {
            where_bitmap::compile_condition(self, brain, &condition.node, &mut ops)?;
            ops.push(BitmapOp::And);
        }
        let ops = optimize_bitmap_ops(ops);
        let max_stack_depth = compute_bitmap_stack_depth(&ops)?;
        Ok(BoundRetrievePlan {
            brain_id: brain,
            task: raw.task.node.value.to_string(),
            mode,
            bitmap_program: BitmapProgram {
                ops,
                max_stack_depth,
            },
            context_policy,
            quality_thresholds: thresholds,
            weights: default_weights(mode),
            diversity_lambda_q16: raw.diversity_lambda_q16.as_ref().map(|s| s.node),
        })
    }

    pub fn bind_verify_fact(
        &self,
        raw: &RawVerifyFact<'_>,
    ) -> Result<BoundVerifyFactPlan, BindError> {
        let brain = self
            .catalog
            .resolve_brain(raw.brain.node.value.as_ref())
            .ok_or(BindError::UnknownBrain)?;
        PolicyValidator::new(self.view)
            .enforce_verify_fact(brain)
            .map_err(BindError::PolicyDenied)?;
        let view = self.view;
        Ok(BoundVerifyFactPlan {
            brain_id: brain,
            fact: raw.fact.node.value.to_string(),
            max_candidates: view.effective_candidate_limit(view.default_candidate_limit),
            max_evidence: view.effective_candidate_limit(8),
        })
    }

    pub fn bind_remember(&self, raw: &RawRemember<'_>) -> Result<BoundRememberPlan, BindError> {
        let scope = self
            .catalog
            .resolve_write_scope(raw.scope.node.value.as_ref())
            .ok_or(BindError::UnknownScope)?;
        let memory_type = raw
            .memory_type
            .node
            .value
            .parse::<MemoryType>()
            .map_err(|_| BindError::InvalidMemoryType)?;
        let ttl_seconds = raw.ttl.as_ref().map(|ttl| match ttl.node {
            TtlValue::Seconds(seconds) => seconds,
        });
        PolicyValidator::new(self.view)
            .enforce_remember(scope, memory_type, ttl_seconds)
            .map_err(BindError::PolicyDenied)?;
        Ok(BoundRememberPlan {
            scope_id: scope,
            scope_name: raw.scope.node.value.to_string(),
            memory_type,
            content: raw.content.node.value.to_string(),
            ttl_seconds,
        })
    }
}
