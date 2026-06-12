use std::collections::BTreeSet;

use crate::verification::numeric::{extract_numeric_values, NumericValue};
use crate::verification::temporal::{extract_temporal_query_range, TemporalQueryRange};

use super::tokenize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum NumericConditionOperator {
    Equal,
    AtLeast,
    AtMost,
    GreaterThan,
    LessThan,
    Between,
}

impl NumericConditionOperator {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Equal => "equal",
            Self::AtLeast => "at_least",
            Self::AtMost => "at_most",
            Self::GreaterThan => "greater_than",
            Self::LessThan => "less_than",
            Self::Between => "between",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QueryNumericCondition {
    pub id: String,
    pub operator: NumericConditionOperator,
    pub values: Vec<NumericValue>,
    pub metric_terms: Vec<String>,
    pub raw_text: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QueryConditionSlot {
    pub id: String,
    pub operator_hint: Option<NumericConditionOperator>,
    pub metric_terms: Vec<String>,
    pub raw_text: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QueryConditionExtraction {
    pub question: String,
    pub numeric_conditions: Vec<QueryNumericCondition>,
    pub condition_slots: Vec<QueryConditionSlot>,
    pub temporal_range: Option<TemporalQueryRange>,
}

impl QueryConditionExtraction {
    pub fn has_structured_conditions(&self) -> bool {
        !self.numeric_conditions.is_empty()
            || !self.condition_slots.is_empty()
            || self.temporal_range.is_some()
    }
}

pub fn extract_query_conditions(question: &str) -> QueryConditionExtraction {
    let question = compact_whitespace(question);
    let numeric_values = extract_numeric_values(&question);
    let words = question.split_whitespace().collect::<Vec<_>>();
    let mut conditions = Vec::new();
    for (index, value) in numeric_values.iter().enumerate() {
        if condition_already_covered(&conditions, value) {
            continue;
        }
        let operator = operator_for_numeric_value(&words, value, &numeric_values);
        let mut values = vec![value.clone()];
        if operator == NumericConditionOperator::Between {
            if let Some(second) = numeric_values.get(index + 1) {
                values.push(second.clone());
            }
        }
        let metric_terms = metric_terms_for_value(&words, value);
        let raw_text = raw_condition_text(&words, value);
        conditions.push(QueryNumericCondition {
            id: format!("n{:02}", conditions.len() + 1),
            operator,
            values,
            metric_terms,
            raw_text,
        });
    }
    QueryConditionExtraction {
        temporal_range: extract_temporal_query_range(&question),
        condition_slots: condition_slots(&question, &conditions),
        question,
        numeric_conditions: conditions,
    }
}

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

fn condition_slots(
    question: &str,
    numeric_conditions: &[QueryNumericCondition],
) -> Vec<QueryConditionSlot> {
    let words = question.split_whitespace().collect::<Vec<_>>();
    let normalized_words = words
        .iter()
        .map(|word| clean_word(word).to_ascii_lowercase())
        .collect::<Vec<_>>();
    let mut slots = Vec::new();
    let mut seen = BTreeSet::new();
    for (index, word) in normalized_words.iter().enumerate() {
        let operator_hint = if matches!(word.as_str(), "minimum" | "min" | "required") {
            Some(NumericConditionOperator::AtLeast)
        } else if matches!(word.as_str(), "maximum" | "max" | "cap" | "limit") {
            Some(NumericConditionOperator::AtMost)
        } else if word == "threshold"
            || word == "retained"
            || (word == "long" && index > 0 && normalized_words[index - 1] == "how")
        {
            Some(NumericConditionOperator::Equal)
        } else {
            None
        };
        let Some(operator_hint) = operator_hint else {
            continue;
        };
        let metric_terms = metric_terms_from_window(&words, index, 5);
        if metric_terms.is_empty()
            || numeric_conditions.iter().any(|condition| {
                condition
                    .metric_terms
                    .iter()
                    .any(|term| metric_terms.contains(term))
            })
        {
            continue;
        }
        let key = metric_terms.join(" ");
        if !seen.insert(key) {
            continue;
        }
        slots.push(QueryConditionSlot {
            id: format!("c{:02}", slots.len() + 1),
            operator_hint: Some(operator_hint),
            metric_terms,
            raw_text: raw_window(&words, index, 4),
        });
    }
    slots
}

fn operator_for_numeric_value(
    words: &[&str],
    value: &NumericValue,
    all_values: &[NumericValue],
) -> NumericConditionOperator {
    let lower = words
        .iter()
        .map(|word| clean_word(word).to_ascii_lowercase())
        .collect::<Vec<_>>();
    if (contains_phrase(&lower, &["between"]) || contains_phrase(&lower, &["from"]))
        && all_values.len() >= 2
    {
        return NumericConditionOperator::Between;
    }
    let Some(index) = word_index_for_value(words, value) else {
        return NumericConditionOperator::Equal;
    };
    let start = index.saturating_sub(4);
    let end = (index + 4).min(lower.len().saturating_sub(1));
    let window = &lower[start..=end];
    if contains_phrase(window, &["at", "least"])
        || contains_phrase(window, &["no", "less", "than"])
        || contains_any(window, &["minimum", "min", "gte", "above"])
    {
        return NumericConditionOperator::AtLeast;
    }
    if contains_any(window, &["over", "greater", "exceeds", "exceeding", "more"]) {
        return NumericConditionOperator::GreaterThan;
    }
    if contains_phrase(window, &["at", "most"])
        || contains_phrase(window, &["no", "more", "than"])
        || contains_any(window, &["maximum", "max", "lte", "below", "under", "cap"])
    {
        return NumericConditionOperator::AtMost;
    }
    if contains_any(window, &["less", "fewer"]) {
        return NumericConditionOperator::LessThan;
    }
    NumericConditionOperator::Equal
}

fn metric_terms_for_value(words: &[&str], value: &NumericValue) -> Vec<String> {
    let Some(index) = word_index_for_value(words, value) else {
        return Vec::new();
    };
    let start = index.saturating_sub(5);
    let end = (index + 5).min(words.len().saturating_sub(1));
    let mut terms = Vec::new();
    let mut seen = BTreeSet::new();
    for word in &words[start..=end] {
        for term in tokenize(&clean_word(word)) {
            if !is_condition_stopword(&term) && seen.insert(term.clone()) {
                terms.push(term);
            }
        }
    }
    for term in tokenize(&value.raw) {
        terms.retain(|candidate| candidate != &term);
    }
    terms
}

fn metric_terms_from_window(words: &[&str], index: usize, radius: usize) -> Vec<String> {
    let start = index.saturating_sub(radius);
    let end = (index + radius).min(words.len().saturating_sub(1));
    let mut terms = Vec::new();
    let mut seen = BTreeSet::new();
    for word in &words[start..=end] {
        for term in tokenize(&clean_word(word)) {
            if !is_condition_stopword(&term) && seen.insert(term.clone()) {
                terms.push(term);
            }
        }
    }
    terms
}

fn raw_condition_text(words: &[&str], value: &NumericValue) -> String {
    let Some(index) = word_index_for_value(words, value) else {
        return value.raw.clone();
    };
    let start = index.saturating_sub(4);
    let end = (index + 4).min(words.len().saturating_sub(1));
    words[start..=end].join(" ")
}

fn raw_window(words: &[&str], index: usize, radius: usize) -> String {
    let start = index.saturating_sub(radius);
    let end = (index + radius).min(words.len().saturating_sub(1));
    words[start..=end].join(" ")
}

fn word_index_for_value(words: &[&str], value: &NumericValue) -> Option<usize> {
    words.iter().position(|word| {
        let clean = clean_word(word).replace(',', "");
        clean == value.raw || clean.starts_with(&value.raw)
    })
}

fn condition_already_covered(conditions: &[QueryNumericCondition], value: &NumericValue) -> bool {
    conditions
        .iter()
        .any(|condition| condition.values.iter().any(|existing| existing == value))
}

fn temporal_range_terms(range: &TemporalQueryRange) -> Vec<String> {
    vec![range.start.as_iso_date(), range.end.as_iso_date()]
}

fn contains_any(words: &[String], needles: &[&str]) -> bool {
    words.iter().any(|word| needles.contains(&word.as_str()))
}

fn contains_phrase(words: &[String], phrase: &[&str]) -> bool {
    if phrase.is_empty() || phrase.len() > words.len() {
        return false;
    }
    words
        .windows(phrase.len())
        .any(|window| window.iter().map(String::as_str).eq(phrase.iter().copied()))
}

fn clean_word(word: &str) -> String {
    word.trim_matches(|ch: char| {
        !ch.is_ascii_alphanumeric() && !matches!(ch, '.' | ',' | '%' | '$' | '€' | '₸' | '-' | '_')
    })
    .to_owned()
}

fn compact_whitespace(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn is_condition_stopword(term: &str) -> bool {
    matches!(
        term,
        "and"
            | "are"
            | "between"
            | "did"
            | "does"
            | "for"
            | "from"
            | "give"
            | "how"
            | "into"
            | "less"
            | "list"
            | "more"
            | "must"
            | "need"
            | "over"
            | "than"
            | "the"
            | "under"
            | "was"
            | "what"
            | "when"
            | "where"
            | "which"
            | "with"
    )
}

#[cfg(test)]
mod tests {
    use super::{condition_payload_bonus, extract_query_conditions, NumericConditionOperator};

    #[test]
    fn extracts_threshold_with_unit_and_metric_terms() {
        let extracted =
            extract_query_conditions("What p95 latency threshold must be under 200 ms?");

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
        let extracted =
            extract_query_conditions("What p95 latency threshold must be under 200 ms?");
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
}
