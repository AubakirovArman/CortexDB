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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConflictRecord {
    pub cell_id: CellId,
    pub fact: String,
    pub source_trust_q16: Q16,
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

    pub fn conflict_index(&self, view: &AgentView) -> Vec<ConflictRecord> {
        let mut records = self
            .snapshot_versions()
            .into_iter()
            .filter(|version| {
                let metadata = CellMetadata::from_payload(&version.payload);
                view.can_read_scope(scope_id(&metadata.scope))
            })
            .flat_map(|version| {
                let source_trust_q16 = source_trust_q16(&version.payload);
                contradiction_facts(&version.payload)
                    .into_iter()
                    .map(move |fact| ConflictRecord {
                        cell_id: version.cell_id,
                        fact,
                        source_trust_q16,
                    })
            })
            .collect::<Vec<_>>();
        records.sort_by_key(|record| (record.fact.clone(), record.cell_id));
        records
    }

    pub fn conflicts_for_fact(&self, fact: &str, view: &AgentView) -> Vec<ConflictRecord> {
        let fact_terms = tokenize(fact);
        self.conflict_index(view)
            .into_iter()
            .filter(|record| contradiction_text_matches(&record.fact, &fact_terms))
            .collect()
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
    contradiction_facts(payload).into_iter().find_map(|fact| {
        contradiction_text_matches(&fact, fact_terms).then_some(fact_terms.len() as u32)
    })
}

fn contradiction_text_matches(value: &str, fact_terms: &[String]) -> bool {
    if fact_terms.is_empty() {
        return false;
    }
    let contradicts_terms = tokenize(value);
    let matched_terms = fact_terms
        .iter()
        .filter(|term| contradicts_terms.contains(term))
        .count();
    matched_terms == fact_terms.len()
}

fn contradiction_facts(payload: &[u8]) -> Vec<String> {
    String::from_utf8_lossy(payload)
        .lines()
        .filter_map(|line| line.trim().strip_prefix("contradicts="))
        .map(str::trim)
        .filter(|fact| !fact.is_empty())
        .map(str::to_owned)
        .collect()
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
