use crate::agent_view::AgentView;
use crate::ast::{DecimalLiteral, Requirement, TtlValue};
use crate::policy::EffectiveRetrievePolicy;
use crate::types::{RetrievalMode, Q16, Q16_ONE};

use super::{BindError, BitmapOp, ContextPolicy, QualityThresholds, RetrievalWeights};

pub fn decimal_to_q16(literal: &DecimalLiteral<'_>) -> Result<Q16, BindError> {
    let Some((whole, fraction)) = literal.raw.split_once('.') else {
        return Err(BindError::InvalidDecimal);
    };
    let whole = whole
        .parse::<u128>()
        .map_err(|_| BindError::InvalidDecimal)?;
    let fraction_value = fraction
        .parse::<u128>()
        .map_err(|_| BindError::InvalidDecimal)?;
    let scale = 10u128
        .checked_pow(u32::try_from(fraction.len()).map_err(|_| BindError::InvalidDecimal)?)
        .ok_or(BindError::InvalidDecimal)?;
    let numerator = whole
        .checked_mul(scale)
        .and_then(|value| value.checked_add(fraction_value))
        .ok_or(BindError::InvalidDecimal)?;
    if numerator > scale {
        return Err(BindError::InvalidDecimal);
    }
    if numerator == scale {
        return Ok(Q16_ONE);
    }
    Ok(((numerator * u128::from(Q16_ONE) + (scale / 2)) / scale) as Q16)
}

pub fn compute_bitmap_stack_depth(ops: &[BitmapOp]) -> Result<usize, BindError> {
    let mut depth = 0usize;
    let mut max_depth = 0usize;
    for op in ops {
        match op {
            BitmapOp::Push(_) | BitmapOp::PushAgentAllowed | BitmapOp::PushLive => depth += 1,
            BitmapOp::And | BitmapOp::Or => {
                if depth < 2 {
                    return Err(BindError::InvalidBitmapProgram);
                }
                depth -= 1;
            }
            BitmapOp::Not => {
                if depth < 1 {
                    return Err(BindError::InvalidBitmapProgram);
                }
            }
        }
        max_depth = max_depth.max(depth);
    }
    (depth == 1)
        .then_some(max_depth)
        .ok_or(BindError::InvalidBitmapProgram)
}

pub fn normalize_weights(weights: RetrievalWeights) -> RetrievalWeights {
    weights_from_parts([
        u32::from(weights.lexical_q16),
        u32::from(weights.semantic_q16),
        u32::from(weights.recency_q16),
        u32::from(weights.trust_q16),
    ])
}

pub fn default_weights(mode: RetrievalMode) -> RetrievalWeights {
    match mode {
        RetrievalMode::Fast => weights_from_parts([55, 10, 25, 10]),
        RetrievalMode::Balanced => weights_from_parts([30, 35, 20, 15]),
        RetrievalMode::Semantic => weights_from_parts([15, 55, 15, 15]),
        RetrievalMode::Audit => weights_from_parts([20, 20, 20, 40]),
    }
}

pub fn context_policy_for_mode(
    mode: RetrievalMode,
    view: &AgentView,
    effective: &EffectiveRetrievePolicy,
) -> ContextPolicy {
    ContextPolicy {
        budget_tokens: effective.context_budget_tokens,
        candidate_limit: effective.candidate_limit,
        require_citations: view.require_citations_by_default || mode == RetrievalMode::Audit,
    }
}

pub(super) fn apply_requirement(
    thresholds: &mut QualityThresholds,
    context: &mut ContextPolicy,
    requirement: &Requirement<'_>,
) -> Result<(), BindError> {
    match requirement {
        Requirement::RequireCitations => context.require_citations = true,
        Requirement::MinConfidence(value) => {
            thresholds.min_confidence_q16 =
                thresholds.min_confidence_q16.max(decimal_to_q16(value)?);
        }
        Requirement::SourceTrust(value) => thresholds.min_source_trust_q16 = decimal_to_q16(value)?,
        Requirement::Freshness(TtlValue::Seconds(seconds)) => {
            thresholds.max_freshness_seconds = Some(*seconds);
        }
    }
    Ok(())
}

fn weights_from_parts(parts: [u32; 4]) -> RetrievalWeights {
    let total: u32 = parts.iter().sum();
    if total == 0 {
        return weights_from_parts([1, 1, 1, 1]);
    }
    let first = ((u64::from(parts[0]) * u64::from(Q16_ONE)) / u64::from(total)) as Q16;
    let second = ((u64::from(parts[1]) * u64::from(Q16_ONE)) / u64::from(total)) as Q16;
    let third = ((u64::from(parts[2]) * u64::from(Q16_ONE)) / u64::from(total)) as Q16;
    let used = u32::from(first) + u32::from(second) + u32::from(third);
    RetrievalWeights {
        lexical_q16: first,
        semantic_q16: second,
        recency_q16: third,
        trust_q16: (u32::from(Q16_ONE) - used) as Q16,
    }
}

impl RetrievalWeights {
    pub fn sum_q16(&self) -> u32 {
        u32::from(self.lexical_q16)
            + u32::from(self.semantic_q16)
            + u32::from(self.recency_q16)
            + u32::from(self.trust_q16)
    }
}
