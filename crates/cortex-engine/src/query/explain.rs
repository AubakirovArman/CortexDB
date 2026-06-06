use cortex_aql::{
    eval_bitmap_program, parse_aql, AgentView, AqlStatement, Binder, BitmapProvider, BrainId,
    Comparator, Condition, Literal, RetrievalMode,
};

use crate::database::{cell_meets_quality_thresholds, CandidateResolver, Database};
use crate::error::{EngineError, EngineResult};

use super::EngineAqlProvider;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AqlExplainReport {
    pub task: String,
    pub brain_id: BrainId,
    pub selected_mode: RetrievalMode,
    pub bitmap_plan: String,
    pub bitmap_ops: Vec<String>,
    pub filters: Vec<AqlExplainFilter>,
    pub candidate_counts: AqlCandidateCounts,
    pub candidate_limit: u32,
    pub budget_tokens: u32,
    pub citations_required: bool,
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
    pub after_bitmap: usize,
    pub after_quality: usize,
    pub returned_limit: usize,
}

impl Database {
    pub fn explain_retrieve_aql(
        &self,
        aql: &str,
        view: &AgentView,
    ) -> EngineResult<AqlExplainReport> {
        let statement = parse_aql(aql).map_err(|error| EngineError::AqlParse(error.to_string()))?;
        let AqlStatement::Explain(inner) = &statement else {
            return Err(EngineError::InvalidOperation);
        };
        let AqlStatement::RetrieveContext(raw) = inner.as_ref() else {
            return Err(EngineError::InvalidOperation);
        };
        let index = self.try_aql_index()?;
        let bound = Binder::new(&index, view).bind_statement(&statement)?;
        let cortex_aql::BoundPlan::Retrieve(plan) = bound else {
            return Err(EngineError::InvalidOperation);
        };
        let provider = EngineAqlProvider::new(index, view);
        let bitmap_candidates = eval_bitmap_program(&plan.bitmap_program, &provider)?;
        let txn = self.read_txn();
        let after_quality = bitmap_candidates
            .iter()
            .filter_map(|candidate| provider.cell_id_for_candidate(*candidate))
            .filter_map(|cell_id| self.get_cell(txn, cell_id))
            .filter(|payload| cell_meets_quality_thresholds(payload, &plan.quality_thresholds))
            .count();
        let candidate_limit = plan.context_policy.candidate_limit as usize;
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
        if let Some(condition) = &raw.where_clause {
            filters.push(AqlExplainFilter {
                kind: "where".to_owned(),
                expression: condition_to_string(&condition.node),
            });
        }
        Ok(AqlExplainReport {
            task: plan.task.clone(),
            brain_id: plan.brain_id,
            selected_mode: plan.mode,
            bitmap_plan: plan.bitmap_program.explain(),
            bitmap_ops: plan
                .bitmap_program
                .ops
                .iter()
                .map(|op| format!("{op:?}"))
                .collect(),
            filters,
            candidate_counts: AqlCandidateCounts {
                universe: provider.universe().len(),
                agent_allowed: provider.agent_allowed().len(),
                live: provider.live().len(),
                after_bitmap: bitmap_candidates.len(),
                after_quality,
                returned_limit: after_quality.min(candidate_limit),
            },
            candidate_limit: plan.context_policy.candidate_limit,
            budget_tokens: plan.context_policy.budget_tokens,
            citations_required: plan.context_policy.require_citations,
        })
    }
}

fn condition_to_string(condition: &Condition<'_>) -> String {
    match condition {
        Condition::Predicate {
            field,
            comparator,
            literal,
        } => format!(
            "{} {} {}",
            field.node.value,
            comparator_to_string(comparator.node),
            literal_to_string(&literal.node)
        ),
        Condition::Not(inner) => format!("NOT ({})", condition_to_string(&inner.node)),
        Condition::And(left, right) => format!(
            "({}) AND ({})",
            condition_to_string(&left.node),
            condition_to_string(&right.node)
        ),
        Condition::Or(left, right) => format!(
            "({}) OR ({})",
            condition_to_string(&left.node),
            condition_to_string(&right.node)
        ),
    }
}

fn comparator_to_string(comparator: Comparator) -> &'static str {
    match comparator {
        Comparator::Eq => "=",
        Comparator::NotEq => "!=",
        Comparator::Gt => ">",
        Comparator::Gte => ">=",
        Comparator::Lt => "<",
        Comparator::Lte => "<=",
        Comparator::In => "IN",
    }
}

fn literal_to_string(literal: &Literal<'_>) -> String {
    match literal {
        Literal::String(value) => format!("{:?}", value.value),
        Literal::Identifier(value) => value.value.to_string(),
        Literal::List(values) => format!(
            "[{}]",
            values
                .iter()
                .map(|value| literal_to_string(&value.node))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Literal::Integer(value) => value.to_string(),
        Literal::Decimal(value) => value.raw.to_string(),
        Literal::Boolean(value) => value.to_string(),
    }
}
