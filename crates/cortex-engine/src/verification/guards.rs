use std::collections::BTreeSet;

use crate::query::CellMetadata;
use crate::search::tokenize;

use super::{VerificationEvidence, VerificationGuard};

pub(super) fn citation_guard(evidence: &VerificationEvidence) -> Option<VerificationGuard> {
    evidence.citation.is_none().then(|| VerificationGuard {
        cell_id: Some(evidence.cell_id),
        code: "missing_citation",
        message: "supporting evidence has no source= or citation=".to_owned(),
    })
}

pub(super) fn numeric_mismatch_guard(
    fact: &str,
    payload: &[u8],
    evidence: &VerificationEvidence,
) -> Option<VerificationGuard> {
    numeric_mismatch(fact, payload).map(|_| VerificationGuard {
        cell_id: Some(evidence.cell_id),
        code: "numeric_mismatch",
        message: "payload numeric claim differs from fact numeric claim".to_owned(),
    })
}

pub(super) fn numeric_mismatch(fact: &str, payload: &[u8]) -> Option<u32> {
    let fact_numbers = numeric_values(fact);
    if fact_numbers.is_empty() {
        return None;
    }
    let metadata = CellMetadata::from_payload(payload);
    let payload_numbers = numeric_values(&metadata.body_text);
    if payload_numbers.is_empty() || same_number_set(&fact_numbers, &payload_numbers) {
        return None;
    }
    let fact_terms = non_numeric_terms(fact);
    let payload_terms = non_numeric_terms(&metadata.body_text);
    let matched = fact_terms
        .iter()
        .filter(|term| payload_terms.contains(term))
        .count();
    (matched > 0).then_some(matched as u32)
}

fn same_number_set(left: &[String], right: &[String]) -> bool {
    left.iter().collect::<BTreeSet<_>>() == right.iter().collect::<BTreeSet<_>>()
}

fn non_numeric_terms(text: &str) -> Vec<String> {
    tokenize(text)
        .into_iter()
        .filter(|term| numeric_values(term).is_empty())
        .collect()
}

fn numeric_values(text: &str) -> Vec<String> {
    let mut values = Vec::new();
    let mut current = String::new();
    for character in text.chars() {
        if character.is_ascii_digit() || matches!(character, '.' | ',' | '_') {
            current.push(character);
        } else {
            push_number(&mut values, &mut current);
        }
    }
    push_number(&mut values, &mut current);
    values
}

fn push_number(values: &mut Vec<String>, current: &mut String) {
    if current.chars().any(|character| character.is_ascii_digit()) {
        values.push(normalize_number(current));
    }
    current.clear();
}

fn normalize_number(value: &str) -> String {
    let mut value = value.replace(['_', ','], "");
    if let Some((whole, fraction)) = value.split_once('.') {
        let whole = trim_leading_zeroes(whole);
        let fraction = fraction.trim_end_matches('0');
        value = if fraction.is_empty() {
            whole
        } else {
            format!("{whole}.{fraction}")
        };
    } else {
        value = trim_leading_zeroes(&value);
    }
    if value.is_empty() {
        "0".to_owned()
    } else {
        value
    }
}

fn trim_leading_zeroes(value: &str) -> String {
    let trimmed = value.trim_start_matches('0');
    if trimmed.is_empty() {
        "0".to_owned()
    } else {
        trimmed.to_owned()
    }
}
