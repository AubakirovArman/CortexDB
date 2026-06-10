use crate::query::CellMetadata;
use crate::search::tokenize;

use super::contradiction::tokenize_support_text;
use super::numeric::{extract_numeric_values, normalized_numeric_equal};
use super::temporal::{extract_temporal_query_range, temporal_validity_from_payload};
use super::VerificationMatchKind;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct VerificationSupportMatch {
    pub matched_terms: u32,
    pub match_score_q16: u16,
    pub match_kind: VerificationMatchKind,
}

pub(super) fn support_match(fact: &str, payload: &[u8]) -> Option<VerificationSupportMatch> {
    let fact_terms = tokenize(fact);
    if fact_terms.is_empty() {
        return None;
    }
    let payload_terms = tokenize_support_text(payload);
    let matched_terms = fact_terms
        .iter()
        .filter(|term| payload_terms.contains(term))
        .count();

    if normalized_body(payload).contains(&normalized_text(fact)) {
        return Some(VerificationSupportMatch {
            matched_terms: fact_terms.len() as u32,
            match_score_q16: u16::MAX,
            match_kind: VerificationMatchKind::ExactText,
        });
    }

    if numeric_entailment(fact, payload) {
        let support_terms = non_numeric_support_terms(fact, payload);
        return Some(VerificationSupportMatch {
            matched_terms: matched_terms.max(support_terms.len()) as u32,
            match_score_q16: 62_000,
            match_kind: VerificationMatchKind::NumericEntailment,
        });
    }

    (matched_terms == fact_terms.len()).then_some(VerificationSupportMatch {
        matched_terms: matched_terms as u32,
        match_score_q16: 60_000,
        match_kind: VerificationMatchKind::SemanticEntailment,
    })
}

pub(super) fn term_coverage_q16(matched_terms: u32, fact_terms: &[String]) -> u16 {
    if fact_terms.is_empty() {
        return 0;
    }
    ((u64::from(matched_terms) * u64::from(u16::MAX)) / fact_terms.len() as u64) as u16
}

fn numeric_entailment(fact: &str, payload: &[u8]) -> bool {
    let fact_values = extract_numeric_values(fact);
    if fact_values.is_empty() {
        return false;
    }
    let payload_text = CellMetadata::from_payload(payload).body_text;
    let payload_values = extract_numeric_values(&payload_text);
    if payload_values.is_empty()
        || !fact_values.iter().all(|fact_value| {
            payload_values
                .iter()
                .any(|payload_value| normalized_numeric_equal(fact_value, payload_value))
        })
    {
        return false;
    }

    let payload_terms = tokenize_support_text(payload);
    let terms = non_numeric_support_terms(fact, payload);
    !terms.is_empty() && terms.iter().all(|term| payload_terms.contains(term))
}

fn non_numeric_terms(text: &str) -> Vec<String> {
    tokenize(text)
        .into_iter()
        .filter(|term| !term.chars().any(|ch| ch.is_ascii_digit()))
        .collect()
}

fn non_numeric_support_terms(fact: &str, payload: &[u8]) -> Vec<String> {
    let has_temporal_validity = extract_temporal_query_range(fact).is_some()
        && !temporal_validity_from_payload(payload).is_empty();
    non_numeric_terms(fact)
        .into_iter()
        .filter(|term| !has_temporal_validity || !is_temporal_connector(term))
        .collect()
}

fn is_temporal_connector(term: &str) -> bool {
    matches!(term, "on" | "in" | "as" | "of" | "at" | "during")
}

fn normalized_body(payload: &[u8]) -> String {
    normalized_text(&CellMetadata::from_payload(payload).body_text)
}

fn normalized_text(text: &str) -> String {
    text.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_text_match_gets_full_score() {
        let matched =
            support_match("ABC budget approved", b"ABC budget approved by board").unwrap();

        assert_eq!(matched.match_kind, VerificationMatchKind::ExactText);
        assert_eq!(matched.match_score_q16, u16::MAX);
    }

    #[test]
    fn numeric_entailment_accepts_normalized_equal_values() {
        let matched = support_match(
            "ABC budget approved for 12,000",
            b"ABC budget approved for 12000.00",
        )
        .unwrap();

        assert_eq!(matched.match_kind, VerificationMatchKind::NumericEntailment);
    }

    #[test]
    fn numeric_entailment_accepts_date_covered_by_validity_metadata() {
        let matched = support_match(
            "Solar Plant budget is 1.2B KZT on 2025-05-01",
            b"valid_from=2025-01-01\nvalid_to=2025-12-31\n\nSolar Plant budget is 1.2B KZT.",
        )
        .unwrap();

        assert_eq!(matched.match_kind, VerificationMatchKind::NumericEntailment);
    }

    #[test]
    fn partial_overlap_is_not_semantic_support() {
        assert!(support_match("ABC budget approved", b"ABC budget delayed").is_none());
    }
}
