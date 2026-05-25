use thiserror::Error;

use crate::agent_view::AgentView;
use crate::ast::{RawRemember, RawRetrieveContext, RawVerifyFact, TtlValue};
use crate::policy::{PolicyError, PolicyValidator};
use crate::types::{BrainId, MemoryType, RetrievalMode, ScopeId, Q16, Q16_ZERO};

mod support;
mod where_bitmap;

use support::apply_requirement;
pub use support::{
    compute_bitmap_stack_depth, context_policy_for_mode, decimal_to_q16, default_weights,
    normalize_weights,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BitmapHandle(pub u64);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QualityThresholds {
    pub min_confidence_q16: Q16,
    pub min_source_trust_q16: Q16,
    pub max_freshness_seconds: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BitmapProgram {
    pub ops: Vec<BitmapOp>,
    pub max_stack_depth: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BitmapOp {
    Push(BitmapHandle),
    PushAgentAllowed,
    PushLive,
    And,
    Or,
    /// Complement is evaluated only inside the segment-local universe.
    Not,
}

pub trait AqlCatalog {
    fn resolve_brain(&self, name: &str) -> Option<BrainId>;
    fn resolve_scope(&self, name: &str) -> Option<ScopeId>;
    fn scope_bitmap(&self, scope: ScopeId) -> Option<BitmapHandle>;
    fn status_bitmap(&self, status: &str) -> Option<BitmapHandle>;
    fn cell_type_bitmap(&self, memory_type: MemoryType) -> Option<BitmapHandle>;
    fn field_is_filterable(&self, field: &str) -> bool;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RetrievalWeights {
    pub lexical_q16: Q16,
    pub semantic_q16: Q16,
    pub recency_q16: Q16,
    pub trust_q16: Q16,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContextPolicy {
    pub budget_tokens: u32,
    pub candidate_limit: u32,
    pub require_citations: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BoundRetrievePlan {
    pub brain_id: BrainId,
    pub task: String,
    pub mode: RetrievalMode,
    pub bitmap_program: BitmapProgram,
    pub context_policy: ContextPolicy,
    pub quality_thresholds: QualityThresholds,
    pub weights: RetrievalWeights,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BoundVerifyFactPlan {
    pub brain_id: BrainId,
    pub fact: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BoundRememberPlan {
    pub scope_id: ScopeId,
    pub memory_type: MemoryType,
    pub content: String,
    pub ttl_seconds: Option<u64>,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum BindError {
    #[error("unknown brain")]
    UnknownBrain,
    #[error("unknown scope")]
    UnknownScope,
    #[error("unknown bitmap")]
    UnknownBitmap,
    #[error("field is not filterable: {0}")]
    FieldNotFilterable(String),
    #[error("unsupported comparator")]
    UnsupportedComparator,
    #[error("unsupported literal")]
    UnsupportedLiteral,
    #[error("invalid memory type")]
    InvalidMemoryType,
    #[error("invalid decimal literal")]
    InvalidDecimal,
    #[error("invalid bitmap program")]
    InvalidBitmapProgram,
    #[error("policy denied: {0:?}")]
    PolicyDenied(PolicyError),
}

pub struct Binder<'a, C> {
    catalog: &'a C,
    view: &'a AgentView,
}

impl<'a, C: AqlCatalog> Binder<'a, C> {
    pub fn new(catalog: &'a C, view: &'a AgentView) -> Self {
        Self { catalog, view }
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
            where_bitmap::compile_condition(self, &condition.node, &mut ops)?;
            ops.push(BitmapOp::And);
        }
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
        Ok(BoundVerifyFactPlan {
            brain_id: brain,
            fact: raw.fact.node.value.to_string(),
        })
    }

    pub fn bind_remember(&self, raw: &RawRemember<'_>) -> Result<BoundRememberPlan, BindError> {
        let scope = self
            .catalog
            .resolve_scope(raw.scope.node.value.as_ref())
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
            memory_type,
            content: raw.content.node.value.to_string(),
            ttl_seconds,
        })
    }
}
