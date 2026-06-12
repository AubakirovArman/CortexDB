use std::collections::BTreeSet;

use crate::verification::numeric::{extract_numeric_values, NumericValue};
use crate::verification::temporal::extract_temporal_query_range;

use super::types::{
    NumericConditionOperator, QueryConditionExtraction, QueryConditionSlot, QueryNumericCondition,
};
use crate::search::tokenize;

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
    let mut terms = metric_terms_from_words(&words[start..=end]);
    for term in tokenize(&value.raw) {
        terms.retain(|candidate| candidate != &term);
    }
    terms
}

fn metric_terms_from_window(words: &[&str], index: usize, radius: usize) -> Vec<String> {
    let start = index.saturating_sub(radius);
    let end = (index + radius).min(words.len().saturating_sub(1));
    metric_terms_from_words(&words[start..=end])
}

fn metric_terms_from_words(words: &[&str]) -> Vec<String> {
    let mut terms = Vec::new();
    let mut seen = BTreeSet::new();
    for word in words {
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
    raw_window(words, index, 4)
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
