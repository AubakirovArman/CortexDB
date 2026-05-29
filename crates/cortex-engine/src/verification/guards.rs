use crate::query::CellMetadata;
use crate::search::tokenize;
use crate::verification::numeric::{extract_numeric_values, numeric_conflict};

use super::{VerificationEvidence, VerificationGuard};

pub(super) fn citation_guard(evidence: &VerificationEvidence) -> Option<VerificationGuard> {
    evidence.citation.is_none().then(|| VerificationGuard {
        cell_id: Some(evidence.cell_id),
        code: crate::verification::VerificationGuardCode::MissingCitation,
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
        code: crate::verification::VerificationGuardCode::NumericMismatch,
        message: "payload numeric claim differs from fact numeric claim".to_owned(),
    })
}

pub(super) fn numeric_mismatch(fact: &str, payload: &[u8]) -> Option<u32> {
    let fact_values = extract_numeric_values(fact);
    if fact_values.is_empty() {
        return None;
    }
    let metadata = CellMetadata::from_payload(payload);
    let payload_values = extract_numeric_values(&metadata.body_text);
    if payload_values.is_empty() {
        return None;
    }

    // Check if any numeric pair conflicts
    let has_conflict = fact_values
        .iter()
        .any(|fv| payload_values.iter().any(|pv| numeric_conflict(fv, pv)));
    if !has_conflict {
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

fn non_numeric_terms(text: &str) -> Vec<String> {
    tokenize(text)
        .into_iter()
        .filter(|term| extract_numeric_values(term).is_empty())
        .collect()
}
