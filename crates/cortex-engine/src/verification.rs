use cortex_aql::{parse_aql, AgentView, Binder, BoundPlan, Q16};
use cortex_core::CellId;

use crate::database::Database;
use crate::error::{EngineError, EngineResult};
use crate::query::{scope_id, CellMetadata};
use crate::search::tokenize;
use crate::source_trust::{SourceTrust, SourceTrustCategory};

mod contradiction;
mod guards;
pub mod numeric;

use contradiction::{
    contradiction_facts, contradiction_match, contradiction_text_matches, tokenize_support_text,
};
use guards::{citation_guard, numeric_mismatch, numeric_mismatch_conflict, numeric_mismatch_guard};

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
    pub source_trust_category: SourceTrustCategory,
    pub citation: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerificationReport {
    pub fact: String,
    pub status: VerificationStatus,
    pub evidence: Vec<VerificationEvidence>,
    pub contradicting_evidence: Vec<VerificationEvidence>,
    pub guards: Vec<VerificationGuard>,
    pub numeric_conflicts: Vec<VerificationNumericConflict>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VerificationGuardCode {
    MissingCitation,
    NumericMismatch,
}

impl VerificationGuardCode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::MissingCitation => "missing_citation",
            Self::NumericMismatch => "numeric_mismatch",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerificationGuard {
    pub cell_id: Option<CellId>,
    pub code: VerificationGuardCode,
    pub message: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerificationNumericConflict {
    pub cell_id: CellId,
    pub metric: String,
    pub left: String,
    pub right: String,
    pub fact_value: numeric::NumericValue,
    pub evidence_value: numeric::NumericValue,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConflictRecord {
    pub cell_id: CellId,
    pub fact: String,
    pub source_trust_q16: Q16,
    pub source_trust_category: SourceTrustCategory,
}

impl Database {
    /// Execute a `VERIFY FACT` AQL statement against stored evidence.
    ///
    /// # Example
    ///
    /// ```
    /// # use cortex_engine::Database;
    /// # use cortex_core::CellId;
    /// # use cortex_aql::{AgentId, AgentView, BrainId, ScopeId};
    /// # let dir = tempfile::tempdir().unwrap();
    /// # let mut db = Database::open(dir.path()).unwrap();
    /// # db.put_cell(CellId(1), b"budget is approved".to_vec()).unwrap();
    /// let view = AgentView {
    ///     agent_id: AgentId(1),
    ///     label: None,
    ///     readable_brains: [BrainId(1)].into_iter().collect(),
    ///     readable_scopes: Default::default(),
    ///     writable_scopes: Default::default(),
    ///     allowed_modes: Default::default(),
    ///     allowed_memory_types: Default::default(),
    ///     max_context_budget_tokens: 10000,
    ///     default_context_budget_tokens: 1000,
    ///     max_candidate_limit: 100,
    ///     default_candidate_limit: 10,
    ///     min_required_confidence_q16: Default::default(),
    ///     max_ttl_seconds: None,
    ///     allow_remember: false,
    ///     allow_verify_fact: true,
    ///     allow_audit_mode: false,
    ///     require_citations_by_default: false,
    ///     private_scope: None,
    /// };
    /// let report = db.verify_fact_aql(r#"VERIFY FACT "budget is approved" IN BRAIN default;"#, &view).unwrap();
    /// assert_eq!(report.fact, "budget is approved");
    /// ```
    pub fn verify_fact_aql(&self, aql: &str, view: &AgentView) -> EngineResult<VerificationReport> {
        let statement = parse_aql(aql).map_err(|error| EngineError::AqlParse(error.to_string()))?;
        let index = self.try_aql_index()?;
        let bound = Binder::new(&index, view).bind_statement(&statement)?;
        let BoundPlan::VerifyFact(plan) = bound else {
            return Err(EngineError::InvalidOperation);
        };
        let mut evidence = Vec::new();
        let mut contradicting_evidence = Vec::new();
        let mut guards = Vec::new();
        let mut numeric_conflicts = Vec::new();
        for version in self.snapshot_versions() {
            if let Some(item) =
                evidence_for_version(version.cell_id, &version.payload, view, &plan.fact)
            {
                if let Some(guard) = citation_guard(&item) {
                    guards.push(guard);
                }
                evidence.push(item);
            }
            if let Some(item) =
                contradiction_for_version(version.cell_id, &version.payload, view, &plan.fact)
            {
                if let Some(guard) = numeric_mismatch_guard(&plan.fact, &version.payload, &item) {
                    guards.push(guard);
                }
                if let Some(conflict) =
                    numeric_mismatch_conflict(&plan.fact, &version.payload, item.cell_id)
                {
                    numeric_conflicts.push(conflict);
                }
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
            guards,
            numeric_conflicts,
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
                let source_trust_category = source_trust(&version.payload).category;
                contradiction_facts(&version.payload)
                    .into_iter()
                    .map(move |fact| ConflictRecord {
                        cell_id: version.cell_id,
                        fact,
                        source_trust_q16,
                        source_trust_category,
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
    fact: &str,
) -> Option<VerificationEvidence> {
    let metadata = CellMetadata::from_payload(payload);
    let fact_terms = tokenize(fact);
    if !view.can_read_scope(scope_id(&metadata.scope)) {
        return None;
    }
    if has_matching_contradiction(payload, &fact_terms) || numeric_mismatch(fact, payload).is_some()
    {
        return None;
    }
    let payload_terms = tokenize_support_text(payload);
    let matched_terms = fact_terms
        .iter()
        .filter(|term| payload_terms.contains(term))
        .count();
    let source_trust = source_trust(payload);
    (matched_terms > 0).then_some(VerificationEvidence {
        cell_id,
        matched_terms: matched_terms as u32,
        source_trust_q16: source_trust.q16,
        source_trust_category: source_trust.category,
        citation: metadata.citation().map(str::to_owned),
    })
}

fn contradiction_for_version(
    cell_id: CellId,
    payload: &[u8],
    view: &AgentView,
    fact: &str,
) -> Option<VerificationEvidence> {
    let metadata = CellMetadata::from_payload(payload);
    let fact_terms = tokenize(fact);
    if !view.can_read_scope(scope_id(&metadata.scope)) || fact_terms.is_empty() {
        return None;
    }
    let source_trust = source_trust(payload);
    numeric_mismatch(fact, payload)
        .or_else(|| contradiction_match(payload, &fact_terms))
        .map(|matched_terms| VerificationEvidence {
            cell_id,
            matched_terms,
            source_trust_q16: source_trust.q16,
            source_trust_category: source_trust.category,
            citation: metadata.citation().map(str::to_owned),
        })
}

fn has_matching_contradiction(payload: &[u8], fact_terms: &[String]) -> bool {
    contradiction_match(payload, fact_terms).is_some()
}

fn source_trust_q16(payload: &[u8]) -> Q16 {
    source_trust(payload).q16
}

fn source_trust(payload: &[u8]) -> SourceTrust {
    SourceTrust::from_q16(CellMetadata::from_payload(payload).source_trust_q16)
}
