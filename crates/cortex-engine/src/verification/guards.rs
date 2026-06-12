use crate::query::CellMetadata;
use crate::search::tokenize;
use cortex_aql::AgentView;
use cortex_core::CellId;

use crate::verification::numeric::{extract_numeric_values, numeric_conflict};
use crate::verification::temporal::{temporal_stale_reason, TemporalStaleReason};

use super::{VerificationEvidence, VerificationGuard, VerificationNumericConflict};

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
    numeric_mismatch_details(fact, payload).map(|details| details.matched_terms)
}

pub(super) fn numeric_mismatch_conflict(
    fact: &str,
    payload: &[u8],
    cell_id: CellId,
) -> Option<VerificationNumericConflict> {
    let details = numeric_mismatch_details(fact, payload)?;
    let left = numeric_display(&details.fact_value);
    let right = numeric_display(&details.evidence_value);
    Some(VerificationNumericConflict {
        cell_id,
        metric: metric_name(payload),
        left,
        right,
        fact_value: details.fact_value,
        evidence_value: details.evidence_value,
    })
}

pub(super) fn stale_fact_guard(
    fact: &str,
    payload: &[u8],
    cell_id: CellId,
    view: &AgentView,
) -> Option<VerificationGuard> {
    let metadata = CellMetadata::from_payload(payload);
    if !view.can_read_scope(crate::query::scope_id(&metadata.scope)) {
        return None;
    }
    let reason = temporal_stale(fact, payload)?;
    Some(VerificationGuard {
        cell_id: Some(cell_id),
        code: crate::verification::VerificationGuardCode::StaleFact,
        message: temporal_stale_message(reason),
    })
}

pub(super) fn temporal_stale(fact: &str, payload: &[u8]) -> Option<TemporalStaleReason> {
    temporal_stale_reason(fact, payload)
}

struct NumericMismatchDetails {
    matched_terms: u32,
    fact_value: crate::verification::numeric::NumericValue,
    evidence_value: crate::verification::numeric::NumericValue,
}

fn numeric_mismatch_details(fact: &str, payload: &[u8]) -> Option<NumericMismatchDetails> {
    let fact_values = extract_numeric_values(fact);
    if fact_values.is_empty() {
        return None;
    }
    let metadata = CellMetadata::from_payload(payload);
    let payload_values = extract_numeric_values(&metadata.body_text);
    if payload_values.is_empty() {
        return None;
    }

    let (fact_value, evidence_value) = fact_values.iter().find_map(|fact_value| {
        let fact_has_equal_evidence = payload_values
            .iter()
            .any(|payload_value| fact_value.normalized_eq(payload_value));
        payload_values
            .iter()
            .find(|payload_value| {
                let evidence_has_equal_fact = fact_values
                    .iter()
                    .any(|other_fact_value| payload_value.normalized_eq(other_fact_value));
                !(fact_has_equal_evidence && evidence_has_equal_fact)
                    && numeric_conflict(fact_value, payload_value)
            })
            .map(|payload_value| (fact_value.clone(), payload_value.clone()))
    })?;

    let fact_terms = non_numeric_terms(fact);
    let payload_terms = non_numeric_terms(&metadata.body_text);
    let matched = fact_terms
        .iter()
        .filter(|term| payload_terms.contains(term))
        .count();
    (matched > 0).then_some(NumericMismatchDetails {
        matched_terms: matched as u32,
        fact_value,
        evidence_value,
    })
}

fn non_numeric_terms(text: &str) -> Vec<String> {
    tokenize(text)
        .into_iter()
        .filter(|term| extract_numeric_values(term).is_empty())
        .collect()
}

fn metric_name(payload: &[u8]) -> String {
    let text = String::from_utf8_lossy(payload);
    text.lines()
        .find_map(|line| line.strip_prefix("metric=").map(str::trim))
        .filter(|value| !value.is_empty())
        .unwrap_or("metric")
        .to_owned()
}

fn numeric_display(value: &crate::verification::numeric::NumericValue) -> String {
    let context = value.currency.as_deref().or(value.unit.as_deref());
    let raw = if context == Some("%") {
        value.raw.trim_end_matches(['%', '.', ','])
    } else {
        value.raw.trim_end_matches(['.', ','])
    };
    match context {
        Some("%") => format!("{raw} %"),
        Some(context)
            if raw == context
                || raw.ends_with(context)
                || raw.to_ascii_uppercase().ends_with(context) =>
        {
            raw.to_owned()
        }
        Some(context) => format!("{raw} {context}"),
        None => raw.to_owned(),
    }
}

fn temporal_stale_message(reason: TemporalStaleReason) -> String {
    match reason {
        TemporalStaleReason::Expired { valid_to } => {
            format!(
                "evidence valid_to={} is before the fact date",
                valid_to.as_iso_date()
            )
        }
        TemporalStaleReason::NotYetValid { valid_from } => {
            format!(
                "evidence valid_from={} is after the fact date",
                valid_from.as_iso_date()
            )
        }
    }
}
