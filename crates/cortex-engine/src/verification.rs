use cortex_aql::{parse_aql, AgentView, Binder, BoundPlan, Q16};
use cortex_core::CellId;

use crate::database::Database;
use crate::error::{EngineError, EngineResult};
use crate::query::{scope_id, CellMetadata};
use crate::search::tokenize;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VerificationStatus {
    Supported,
    Insufficient,
    Contradicted,
    Mixed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerificationEvidence {
    pub cell_id: CellId,
    pub matched_terms: u32,
    pub source_trust_q16: Q16,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerificationReport {
    pub fact: String,
    pub status: VerificationStatus,
    pub evidence: Vec<VerificationEvidence>,
    pub contradicting_evidence: Vec<VerificationEvidence>,
}

impl Database {
    pub fn verify_fact_aql(&self, aql: &str, view: &AgentView) -> EngineResult<VerificationReport> {
        let statement = parse_aql(aql).map_err(|error| EngineError::AqlParse(error.to_string()))?;
        let index = self.try_aql_index()?;
        let bound = Binder::new(&index, view).bind_statement(&statement)?;
        let BoundPlan::VerifyFact(plan) = bound else {
            return Err(EngineError::InvalidOperation);
        };
        let fact_terms = tokenize(&plan.fact);
        let mut evidence = Vec::new();
        let mut contradicting_evidence = Vec::new();
        for version in self.snapshot_versions() {
            if let Some(item) =
                evidence_for_version(version.cell_id, &version.payload, view, &fact_terms)
            {
                evidence.push(item);
            }
            if let Some(item) =
                contradiction_for_version(version.cell_id, &version.payload, view, &fact_terms)
            {
                contradicting_evidence.push(item);
            }
        }
        sort_evidence(&mut evidence);
        sort_evidence(&mut contradicting_evidence);
        evidence.truncate(8);
        contradicting_evidence.truncate(8);
        let status = verification_status(!evidence.is_empty(), !contradicting_evidence.is_empty());
        Ok(VerificationReport {
            fact: plan.fact,
            status,
            evidence,
            contradicting_evidence,
        })
    }
}

fn verification_status(has_support: bool, has_contradiction: bool) -> VerificationStatus {
    match (has_support, has_contradiction) {
        (true, true) => VerificationStatus::Mixed,
        (true, false) => VerificationStatus::Supported,
        (false, true) => VerificationStatus::Contradicted,
        (false, false) => VerificationStatus::Insufficient,
    }
}

fn sort_evidence(evidence: &mut [VerificationEvidence]) {
    evidence.sort_by_key(|item| {
        (
            std::cmp::Reverse(item.matched_terms),
            std::cmp::Reverse(item.source_trust_q16),
            item.cell_id,
        )
    });
}

fn evidence_for_version(
    cell_id: CellId,
    payload: &[u8],
    view: &AgentView,
    fact_terms: &[String],
) -> Option<VerificationEvidence> {
    let metadata = CellMetadata::from_payload(payload);
    if !view.can_read_scope(scope_id(&metadata.scope)) {
        return None;
    }
    if has_matching_contradiction(payload, fact_terms) {
        return None;
    }
    let payload_terms = tokenize_support_text(payload);
    let matched_terms = fact_terms
        .iter()
        .filter(|term| payload_terms.contains(term))
        .count();
    (matched_terms > 0).then_some(VerificationEvidence {
        cell_id,
        matched_terms: matched_terms as u32,
        source_trust_q16: source_trust_q16(payload),
    })
}

fn contradiction_for_version(
    cell_id: CellId,
    payload: &[u8],
    view: &AgentView,
    fact_terms: &[String],
) -> Option<VerificationEvidence> {
    let metadata = CellMetadata::from_payload(payload);
    if !view.can_read_scope(scope_id(&metadata.scope)) || fact_terms.is_empty() {
        return None;
    }
    let source_trust_q16 = source_trust_q16(payload);
    contradiction_match(payload, fact_terms).map(|matched_terms| VerificationEvidence {
        cell_id,
        matched_terms,
        source_trust_q16,
    })
}

fn has_matching_contradiction(payload: &[u8], fact_terms: &[String]) -> bool {
    contradiction_match(payload, fact_terms).is_some()
}

fn contradiction_match(payload: &[u8], fact_terms: &[String]) -> Option<u32> {
    if fact_terms.is_empty() {
        return None;
    }
    String::from_utf8_lossy(payload).lines().find_map(|line| {
        let value = line.trim().strip_prefix("contradicts=")?;
        let contradicts_terms = tokenize(value);
        let matched_terms = fact_terms
            .iter()
            .filter(|term| contradicts_terms.contains(term))
            .count();
        (matched_terms == fact_terms.len()).then_some(matched_terms as u32)
    })
}

fn tokenize_support_text(payload: &[u8]) -> Vec<String> {
    let text = String::from_utf8_lossy(payload);
    tokenize(
        &text
            .lines()
            .filter(|line| !line.trim().starts_with("contradicts="))
            .collect::<Vec<_>>()
            .join("\n"),
    )
}

fn source_trust_q16(payload: &[u8]) -> Q16 {
    let text = String::from_utf8_lossy(payload);
    text.lines()
        .find_map(|line| line.strip_prefix("source_trust_q16="))
        .and_then(|value| value.trim().parse::<u16>().ok())
        .unwrap_or(32_768)
}
