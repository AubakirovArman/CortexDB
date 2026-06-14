use std::cmp::Reverse;

use cortex_aql::{AgentView, Q16};
use cortex_core::memtable::CellVersion;

use super::contradiction::contradiction_match;
use super::graph::is_graph_contradiction_payload;
use super::guards::{numeric_mismatch, temporal_stale_from_metadata};
use super::support::{support_match, term_coverage_q16};
use super::{VerificationEvidence, VerificationMatchKind, VerificationStatus};
use crate::plan::PolicyRewrite;
use crate::query::{scope_id, CellMetadata};
use crate::search::tokenize;
use crate::source_trust::SourceTrust;

pub(super) struct MaterializedVersion<'a> {
    pub(super) version: &'a CellVersion,
    pub(super) payload: Vec<u8>,
}

pub(super) fn version_contains_any_term(version: &CellVersion, fact_terms: &[String]) -> bool {
    if fact_terms.is_empty() {
        return true;
    }
    let metadata = CellMetadata::from_version(version);
    let terms = metadata.weighted_lexical_terms();
    fact_terms.iter().any(|term| terms.contains_key(term))
}

pub(super) fn verification_status(
    has_support: bool,
    has_contradiction: bool,
) -> VerificationStatus {
    match (has_support, has_contradiction) {
        (true, true) => VerificationStatus::Mixed,
        (true, false) => VerificationStatus::Supported,
        (false, true) => VerificationStatus::Contradicted,
        (false, false) => VerificationStatus::Insufficient,
    }
}

pub(super) fn verification_confidence_q16(
    status: VerificationStatus,
    evidence: &[VerificationEvidence],
    contradicting_evidence: &[VerificationEvidence],
) -> Q16 {
    let support = best_evidence_score_q16(evidence);
    let contradiction = best_evidence_score_q16(contradicting_evidence);
    match status {
        VerificationStatus::Supported => support,
        VerificationStatus::Contradicted => contradiction,
        VerificationStatus::Mixed => support.min(contradiction),
        VerificationStatus::Insufficient => 0,
    }
}

pub(super) fn sort_evidence(evidence: &mut [VerificationEvidence]) {
    evidence.sort_by_key(|item| {
        (
            Reverse(item.matched_terms),
            Reverse(item.match_score_q16),
            Reverse(item.source_trust_q16),
            item.cell_id,
        )
    });
}

pub(super) fn evidence_for_version(
    candidate: &MaterializedVersion<'_>,
    view: &AgentView,
    fact: &str,
) -> Option<VerificationEvidence> {
    let version = candidate.version;
    let payload = candidate.payload.as_slice();
    let metadata = CellMetadata::from_payload_with_descriptor(payload, &version.descriptor);
    let fact_terms = tokenize(fact);
    if !PolicyRewrite::allows_scope(view, scope_id(&metadata.scope)) {
        return None;
    }
    if is_graph_contradiction_payload(payload) {
        return None;
    }
    if has_matching_contradiction(payload, &fact_terms)
        || numeric_mismatch(fact, payload).is_some()
        || temporal_stale_from_metadata(fact, &metadata).is_some()
    {
        return None;
    }
    let source_trust = source_trust_from_metadata(&metadata);
    let support = support_match(fact, payload)?;
    Some(VerificationEvidence {
        cell_id: version.cell_id,
        matched_terms: support.matched_terms,
        match_score_q16: support.match_score_q16,
        match_kind: support.match_kind,
        source_trust_q16: source_trust.q16,
        source_trust_category: source_trust.category,
        citation: metadata.citation().map(str::to_owned),
    })
}

pub(super) fn contradiction_for_version(
    candidate: &MaterializedVersion<'_>,
    view: &AgentView,
    fact: &str,
) -> Option<VerificationEvidence> {
    let version = candidate.version;
    let payload = candidate.payload.as_slice();
    let metadata = CellMetadata::from_payload_with_descriptor(payload, &version.descriptor);
    let fact_terms = tokenize(fact);
    if !PolicyRewrite::allows_scope(view, scope_id(&metadata.scope)) || fact_terms.is_empty() {
        return None;
    }
    if temporal_stale_from_metadata(fact, &metadata).is_some() {
        return None;
    }
    let source_trust = source_trust_from_metadata(&metadata);
    if let Some(matched_terms) = numeric_mismatch(fact, payload) {
        return Some(VerificationEvidence {
            cell_id: version.cell_id,
            matched_terms,
            match_score_q16: u16::MAX,
            match_kind: VerificationMatchKind::NumericContradiction,
            source_trust_q16: source_trust.q16,
            source_trust_category: source_trust.category,
            citation: metadata.citation().map(str::to_owned),
        });
    }
    contradiction_match(payload, &fact_terms).map(|matched_terms| VerificationEvidence {
        cell_id: version.cell_id,
        matched_terms,
        match_score_q16: term_coverage_q16(matched_terms, &fact_terms),
        match_kind: VerificationMatchKind::SemanticContradiction,
        source_trust_q16: source_trust.q16,
        source_trust_category: source_trust.category,
        citation: metadata.citation().map(str::to_owned),
    })
}

fn best_evidence_score_q16(evidence: &[VerificationEvidence]) -> Q16 {
    evidence
        .iter()
        .map(|item| item.match_score_q16.min(item.source_trust_q16))
        .max()
        .unwrap_or(0)
}

fn has_matching_contradiction(payload: &[u8], fact_terms: &[String]) -> bool {
    contradiction_match(payload, fact_terms).is_some()
}

fn source_trust_from_metadata(metadata: &CellMetadata) -> SourceTrust {
    SourceTrust::from_metadata(metadata.source_trust_q16, metadata.source_trust_class)
}
