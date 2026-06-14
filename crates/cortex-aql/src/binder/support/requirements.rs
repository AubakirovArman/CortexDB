use crate::ast::{Requirement, TtlValue};

use super::{date, decimal_to_q16, BindError, ContextPolicy, QualityThresholds};

pub(crate) fn apply_requirement(
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
        Requirement::ValidAt(date) => {
            date::validate_iso_date(date.value.as_ref())?;
            thresholds.valid_at = Some(date.value.to_string());
        }
    }
    Ok(())
}
