use cortex_aql::{AgentView, BoundPlan, Q16};
use cortex_core::CellId;

use crate::database::Database;
use crate::error::{EngineError, EngineResult};
use crate::query::cache::AqlStatementKind;
use crate::query::{scope_id, CellMetadata};
use crate::search::tokenize;
use crate::source_trust::{SourceTrust, SourceTrustCategory};

mod conflict_index;
mod contradiction;
pub mod export;
mod graph;
mod guards;
pub mod numeric;
mod support;
pub mod temporal;
mod temporal_index;

pub use conflict_index::{ConflictRecord, ContradictionRelationOptions};
use contradiction::contradiction_match;
pub use export::VerificationReportExportFormat;
use graph::{
    add_graph_relation_contradictions, enrich_evidence_from_source_support_edges,
    is_graph_contradiction_payload,
};
use guards::{
    citation_guard, numeric_mismatch, numeric_mismatch_conflict, numeric_mismatch_guard,
    stale_fact_guard, temporal_stale,
};
pub use numeric::{
    compare_numeric_values, format_scaled_value, normalized_numeric_equal, parse_currency_code,
    parse_magnitude_suffix, parse_unit_code, Magnitude, NumericComparison, NumericValue,
};
use support::{support_match, term_coverage_q16};
pub use temporal::{
    extract_temporal_query_range, parse_temporal_date, TemporalDate, TemporalQueryRange,
    TemporalStaleReason, TemporalValidity,
};
pub use temporal_index::{TemporalFactIndex, TemporalFactKey, TemporalFactRecord};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VerificationStatus {
    Supported,
    Insufficient,
    Contradicted,
    Mixed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VerificationMatchKind {
    ExactText,
    SemanticEntailment,
    NumericEntailment,
    SemanticContradiction,
    NumericContradiction,
    GraphContradiction,
}

impl VerificationMatchKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ExactText => "exact_text",
            Self::SemanticEntailment => "semantic_entailment",
            Self::NumericEntailment => "numeric_entailment",
            Self::SemanticContradiction => "semantic_contradiction",
            Self::NumericContradiction => "numeric_contradiction",
            Self::GraphContradiction => "graph_contradiction",
        }
    }
}

impl VerificationStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Supported => "supported",
            Self::Insufficient => "insufficient",
            Self::Contradicted => "contradicted",
            Self::Mixed => "mixed_evidence",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerificationEvidence {
    pub cell_id: CellId,
    pub matched_terms: u32,
    pub match_score_q16: Q16,
    pub match_kind: VerificationMatchKind,
    pub source_trust_q16: Q16,
    pub source_trust_category: SourceTrustCategory,
    pub citation: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerificationReport {
    pub fact: String,
    pub status: VerificationStatus,
    pub confidence_q16: Q16,
    pub evidence: Vec<VerificationEvidence>,
    pub contradicting_evidence: Vec<VerificationEvidence>,
    pub guards: Vec<VerificationGuard>,
    pub numeric_conflicts: Vec<VerificationNumericConflict>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VerificationGuardCode {
    MissingCitation,
    NumericMismatch,
    StaleFact,
}

impl VerificationGuardCode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::MissingCitation => "missing_citation",
            Self::NumericMismatch => "numeric_mismatch",
            Self::StaleFact => "stale_fact",
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
        let (cached, _) = self.bind_aql_cached(aql, view)?;
        if cached.statement_kind != AqlStatementKind::VerifyFact {
            return Err(EngineError::InvalidOperation);
        }
        let BoundPlan::VerifyFact(plan) = cached.bound_plan else {
            return Err(EngineError::InvalidOperation);
        };
        let mut evidence = Vec::new();
        let mut contradicting_evidence = Vec::new();
        let mut guards = Vec::new();
        let mut numeric_conflicts = Vec::new();
        for version in self.snapshot_versions() {
            if let Some(guard) =
                stale_fact_guard(&plan.fact, &version.payload, version.cell_id, view)
            {
                guards.push(guard);
            }
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
        add_graph_relation_contradictions(self, &plan.fact, view, &mut contradicting_evidence);
        enrich_evidence_from_source_support_edges(self, view, &mut evidence);
        sort_evidence(&mut evidence);
        sort_evidence(&mut contradicting_evidence);
        evidence.truncate(8);
        contradicting_evidence.truncate(8);
        let status = verification_status(!evidence.is_empty(), !contradicting_evidence.is_empty());
        let confidence_q16 =
            verification_confidence_q16(status, &evidence, &contradicting_evidence);
        Ok(VerificationReport {
            fact: plan.fact,
            status,
            confidence_q16,
            evidence,
            contradicting_evidence,
            guards,
            numeric_conflicts,
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

fn verification_confidence_q16(
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

fn best_evidence_score_q16(evidence: &[VerificationEvidence]) -> Q16 {
    evidence
        .iter()
        .map(|item| item.match_score_q16.min(item.source_trust_q16))
        .max()
        .unwrap_or(0)
}

fn sort_evidence(evidence: &mut [VerificationEvidence]) {
    evidence.sort_by_key(|item| {
        (
            std::cmp::Reverse(item.matched_terms),
            std::cmp::Reverse(item.match_score_q16),
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
    if is_graph_contradiction_payload(payload) {
        return None;
    }
    if has_matching_contradiction(payload, &fact_terms)
        || numeric_mismatch(fact, payload).is_some()
        || temporal_stale(fact, payload).is_some()
    {
        return None;
    }
    let source_trust = source_trust(payload);
    let support = support_match(fact, payload)?;
    Some(VerificationEvidence {
        cell_id,
        matched_terms: support.matched_terms,
        match_score_q16: support.match_score_q16,
        match_kind: support.match_kind,
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
    if temporal_stale(fact, payload).is_some() {
        return None;
    }
    let source_trust = source_trust(payload);
    if let Some(matched_terms) = numeric_mismatch(fact, payload) {
        return Some(VerificationEvidence {
            cell_id,
            matched_terms,
            match_score_q16: u16::MAX,
            match_kind: VerificationMatchKind::NumericContradiction,
            source_trust_q16: source_trust.q16,
            source_trust_category: source_trust.category,
            citation: metadata.citation().map(str::to_owned),
        });
    }
    contradiction_match(payload, &fact_terms).map(|matched_terms| VerificationEvidence {
        cell_id,
        matched_terms,
        match_score_q16: term_coverage_q16(matched_terms, &fact_terms),
        match_kind: VerificationMatchKind::SemanticContradiction,
        source_trust_q16: source_trust.q16,
        source_trust_category: source_trust.category,
        citation: metadata.citation().map(str::to_owned),
    })
}

fn has_matching_contradiction(payload: &[u8], fact_terms: &[String]) -> bool {
    contradiction_match(payload, fact_terms).is_some()
}

fn source_trust(payload: &[u8]) -> SourceTrust {
    let metadata = CellMetadata::from_payload(payload);
    SourceTrust::from_metadata(metadata.source_trust_q16, metadata.source_trust_class)
}
