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
        let mut evidence = self
            .snapshot_versions()
            .into_iter()
            .filter_map(|version| {
                evidence_for_version(version.cell_id, &version.payload, view, &fact_terms)
            })
            .collect::<Vec<_>>();
        evidence.sort_by_key(|item| {
            (
                std::cmp::Reverse(item.matched_terms),
                std::cmp::Reverse(item.source_trust_q16),
                item.cell_id,
            )
        });
        evidence.truncate(8);
        let status = if evidence.is_empty() {
            VerificationStatus::Insufficient
        } else {
            VerificationStatus::Supported
        };
        Ok(VerificationReport {
            fact: plan.fact,
            status,
            evidence,
        })
    }
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
    let payload_terms = tokenize(&String::from_utf8_lossy(payload));
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

fn source_trust_q16(payload: &[u8]) -> Q16 {
    let text = String::from_utf8_lossy(payload);
    text.lines()
        .find_map(|line| line.strip_prefix("source_trust_q16="))
        .and_then(|value| value.trim().parse::<u16>().ok())
        .unwrap_or(32_768)
}
