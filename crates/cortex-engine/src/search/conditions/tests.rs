use super::{condition_payload_bonus, extract_query_conditions, NumericConditionOperator};

#[test]
fn extracts_threshold_with_unit_and_metric_terms() {
    let extracted = extract_query_conditions("What p95 latency threshold must be under 200 ms?");

    assert_eq!(extracted.numeric_conditions.len(), 1);
    assert_eq!(
        extracted.numeric_conditions[0].operator,
        NumericConditionOperator::AtMost
    );
    assert_eq!(
        extracted.numeric_conditions[0].values[0].unit,
        Some("ms".to_owned())
    );
    assert!(extracted.numeric_conditions[0]
        .metric_terms
        .contains(&"latency".to_owned()));
}

#[test]
fn extracts_between_range() {
    let extracted = extract_query_conditions("Which score range is between 40 and 60?");

    assert_eq!(extracted.numeric_conditions.len(), 1);
    assert_eq!(
        extracted.numeric_conditions[0].operator,
        NumericConditionOperator::Between
    );
    assert_eq!(extracted.numeric_conditions[0].values.len(), 2);
}

#[test]
fn payload_bonus_rewards_matching_numeric_condition() {
    let extracted = extract_query_conditions("What p95 latency threshold must be under 200 ms?");
    let matched = condition_payload_bonus(
        &extracted,
        b"p95 latency threshold is 180 ms for the EU route.",
    );
    let weak = condition_payload_bonus(
        &extracted,
        b"p95 latency threshold is 280 ms for the EU route.",
    );

    assert!(matched > weak);
}

#[test]
fn extracts_temporal_range() {
    let extracted = extract_query_conditions("What changed after 2026-04-01?");

    assert!(extracted.temporal_range.is_some());
    assert!(extracted.has_structured_conditions());
}

#[test]
fn extracts_metric_only_condition_slots() {
    let extracted = extract_query_conditions(
        "What minimum KV-cache hit-rate and max sequence length are required?",
    );

    assert!(extracted.numeric_conditions.is_empty());
    assert!(!extracted.condition_slots.is_empty());
    assert!(extracted
        .condition_slots
        .iter()
        .any(|slot| slot.metric_terms.contains(&"sequence".to_owned())));
}

#[test]
fn payload_bonus_rewards_metric_only_condition_slots() {
    let extracted = extract_query_conditions(
        "What minimum KV-cache hit-rate and max sequence length are required?",
    );
    let matched = condition_payload_bonus(
        &extracted,
        b"KV cache hit rate must be high and sequence length is capped.",
    );
    let weak = condition_payload_bonus(&extracted, b"General model routing notes.");

    assert!(matched > weak);
}
