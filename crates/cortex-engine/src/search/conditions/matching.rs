use std::collections::BTreeSet;

use crate::search::tokenize;
use crate::verification::numeric::{extract_numeric_values, NumericValue};
use crate::verification::temporal::TemporalQueryRange;

use super::types::{
    NumericConditionOperator, QueryConditionExtraction, QueryConditionSlot, QueryNumericCondition,
};

pub fn condition_payload_bonus(extraction: &QueryConditionExtraction, payload: &[u8]) -> u64 {
    if !extraction.has_structured_conditions() {
        return 0;
    }
    let payload_text = String::from_utf8_lossy(payload);
    let payload_values = extract_numeric_values(&payload_text);
    let payload_terms = tokenize(&payload_text).into_iter().collect::<BTreeSet<_>>();
    let mut bonus = 0u64;
    for condition in &extraction.numeric_conditions {
        if condition_matches_payload(condition, &payload_values, &payload_terms) {
            bonus = bonus.saturating_add(18_000);
        } else if metric_terms_overlap(condition, &payload_terms) {
            bonus = bonus.saturating_add(4_000);
        }
    }
    for slot in &extraction.condition_slots {
        if slot_terms_overlap(slot, &payload_terms) {
            bonus = bonus.saturating_add(6_000);
        }
    }
    if let Some(range) = &extraction.temporal_range {
        let range_terms = temporal_range_terms(range);
        if range_terms.iter().any(|term| payload_text.contains(term)) {
            bonus = bonus.saturating_add(8_000);
        }
    }
    bonus
}

fn condition_matches_payload(
    condition: &QueryNumericCondition,
    payload_values: &[NumericValue],
    payload_terms: &BTreeSet<String>,
) -> bool {
    payload_values.iter().any(|candidate| {
        compatible_numeric_context(&condition.values[0], candidate)
            && value_satisfies_condition(condition, candidate)
            && (condition.metric_terms.is_empty() || metric_terms_overlap(condition, payload_terms))
    })
}

fn value_satisfies_condition(condition: &QueryNumericCondition, candidate: &NumericValue) -> bool {
    let first = &condition.values[0];
    match condition.operator {
        NumericConditionOperator::Equal => candidate.scaled_value == first.scaled_value,
        NumericConditionOperator::AtLeast => candidate.scaled_value >= first.scaled_value,
        NumericConditionOperator::GreaterThan => candidate.scaled_value > first.scaled_value,
        NumericConditionOperator::AtMost => candidate.scaled_value <= first.scaled_value,
        NumericConditionOperator::LessThan => candidate.scaled_value < first.scaled_value,
        NumericConditionOperator::Between => {
            let Some(second) = condition.values.get(1) else {
                return candidate.scaled_value == first.scaled_value;
            };
            let low = first.scaled_value.min(second.scaled_value);
            let high = first.scaled_value.max(second.scaled_value);
            candidate.scaled_value >= low && candidate.scaled_value <= high
        }
    }
}

fn compatible_numeric_context(query: &NumericValue, candidate: &NumericValue) -> bool {
    match (&query.currency, &candidate.currency) {
        (Some(left), Some(right)) if left != right => return false,
        (Some(_), None) => return false,
        _ => {}
    }
    match (&query.unit, &candidate.unit) {
        (Some(left), Some(right)) if left != right => return false,
        (Some(_), None) => return false,
        _ => {}
    }
    true
}

fn metric_terms_overlap(
    condition: &QueryNumericCondition,
    payload_terms: &BTreeSet<String>,
) -> bool {
    condition
        .metric_terms
        .iter()
        .filter(|term| term.len() >= 3)
        .any(|term| payload_terms.contains(term))
}

fn slot_terms_overlap(slot: &QueryConditionSlot, payload_terms: &BTreeSet<String>) -> bool {
    slot.metric_terms
        .iter()
        .filter(|term| term.len() >= 3)
        .any(|term| payload_terms.contains(term))
}

fn temporal_range_terms(range: &TemporalQueryRange) -> Vec<String> {
    vec![range.start.as_iso_date(), range.end.as_iso_date()]
}
