use std::collections::BTreeSet;

use cortex_aql::{AgentView, BoundPlan};
use cortex_core::memtable::{CellVersion, ReadTxn};

use super::evidence::{
    contradiction_for_version, evidence_for_version, sort_evidence, verification_confidence_q16,
    verification_status, version_contains_any_term, MaterializedVersion,
};
use super::graph::{
    add_graph_relation_contradictions_from_versions, enrich_evidence_from_source_support_edges,
};
use super::guards::{
    citation_guard, numeric_mismatch_conflict, numeric_mismatch_guard, stale_fact_guard,
};
use super::{VerificationEvidence, VerificationReport};
use crate::database::Database;
use crate::error::{EngineError, EngineResult};
use crate::options::PayloadResidency;
use crate::query::cache::AqlStatementKind;
use crate::query::CellMetadata;
use crate::search::tokenize;

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
        let cached = self.bind_verify_fact_cached(aql, view)?;
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
        let pin = self.pin_read_txn();
        let txn = pin.read_txn();
        let candidate_versions = self.materialize_verification_versions(
            self.verification_candidate_versions(&plan.fact, txn)?,
        )?;
        for candidate in &candidate_versions {
            let version = candidate.version;
            let payload = candidate.payload.as_slice();
            if let Some(guard) = stale_fact_guard(&plan.fact, version, view) {
                guards.push(guard);
            }
            if let Some(item) = evidence_for_version(candidate, view, &plan.fact) {
                if let Some(guard) = citation_guard(&item) {
                    guards.push(guard);
                }
                evidence.push(item);
            }
            if let Some(item) = contradiction_for_version(candidate, view, &plan.fact) {
                if let Some(guard) = numeric_mismatch_guard(&plan.fact, payload, &item) {
                    guards.push(guard);
                }
                if let Some(conflict) = numeric_mismatch_conflict(&plan.fact, payload, item.cell_id)
                {
                    numeric_conflicts.push(conflict);
                }
                contradicting_evidence.push(item);
            }
        }
        add_graph_relation_contradictions_from_versions(
            &candidate_versions,
            &plan.fact,
            view,
            &mut contradicting_evidence,
        );
        let support_versions = self.verification_source_support_versions(&evidence, view, txn)?;
        enrich_evidence_from_source_support_edges(self, &support_versions, view, &mut evidence);
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

    fn verification_candidate_versions<'a>(
        &'a self,
        fact: &str,
        txn: ReadTxn,
    ) -> EngineResult<Vec<&'a CellVersion>> {
        let fact_terms = tokenize(fact);
        if fact_terms.is_empty() {
            return Ok(self.memtable.visible_iter(txn).collect());
        }

        let mut cell_ids = BTreeSet::new();
        if !self.manifest().live_segments.is_empty() {
            let persisted = self.persisted_index_state_cached()?;
            for term in &fact_terms {
                if let Some(term_candidates) = persisted.lexical.terms.get(term) {
                    cell_ids.extend(
                        term_candidates
                            .iter()
                            .filter_map(|candidate| persisted.candidate_to_cell.get(candidate))
                            .copied(),
                    );
                }
            }
        }

        let checkpoint_seq = cortex_core::CommitSeq(self.manifest().checkpoint_seq);
        let changed = self
            .memtable
            .changed_cell_ids_after(checkpoint_seq)
            .into_iter()
            .collect::<BTreeSet<_>>();
        let scan_all_live = self.manifest().live_segments.is_empty();
        if scan_all_live {
            for version in self.memtable.visible_iter(txn) {
                if version_contains_any_term(version, &fact_terms) {
                    cell_ids.insert(version.cell_id);
                }
            }
        } else {
            for cell_id in changed {
                let Some(version) = self.memtable.read(txn, cell_id) else {
                    continue;
                };
                if version_contains_any_term(version, &fact_terms) {
                    cell_ids.insert(version.cell_id);
                }
            }
        }

        if cell_ids.is_empty() && scan_all_live {
            return Ok(self.memtable.visible_iter(txn).collect());
        }
        if cell_ids.is_empty() {
            for version in self.memtable.visible_iter(txn).take(32) {
                cell_ids.insert(version.cell_id);
            }
        }

        let mut versions = cell_ids
            .into_iter()
            .filter_map(|cell_id| self.memtable.read(txn, cell_id))
            .collect::<Vec<_>>();
        versions.sort_by_key(|version| version.cell_id);
        versions.dedup_by_key(|version| version.cell_id);
        Ok(versions)
    }

    fn materialize_verification_versions<'a>(
        &'a self,
        versions: Vec<&'a CellVersion>,
    ) -> EngineResult<Vec<MaterializedVersion<'a>>> {
        versions
            .into_iter()
            .map(|version| {
                self.payload_for_version(version)
                    .map(|payload| MaterializedVersion { version, payload })
            })
            .collect()
    }

    fn verification_source_support_versions<'a>(
        &'a self,
        evidence: &[VerificationEvidence],
        view: &AgentView,
        txn: ReadTxn,
    ) -> EngineResult<Vec<MaterializedVersion<'a>>> {
        if evidence.is_empty() {
            return Ok(Vec::new());
        }
        let evidence_ids = evidence
            .iter()
            .map(|item| item.cell_id)
            .map(|cell_id| format!("cell:{}", cell_id.0))
            .collect::<Vec<_>>();
        let checkpoint_seq = cortex_core::CommitSeq(self.manifest().checkpoint_seq);
        let scan_all_live = self.manifest().live_segments.is_empty()
            || self.payload_residency == PayloadResidency::Lazy;
        let visible = if scan_all_live {
            self.memtable.visible_iter(txn).collect::<Vec<_>>()
        } else {
            self.memtable
                .visible_created_after_iter(txn, checkpoint_seq)
                .collect::<Vec<_>>()
        };
        let mut support_versions = Vec::new();
        for version in visible {
            let metadata = CellMetadata::from_version(version);
            if metadata.cell_type != "relation" {
                continue;
            }
            if !view.can_read_scope(crate::query::scope_id(&metadata.scope)) {
                continue;
            }
            let payload = self.payload_for_version(version)?;
            let is_source_support = evidence_ids.iter().any(|id| {
                std::str::from_utf8(&payload)
                    .map(|payload| payload.contains(id))
                    .unwrap_or(false)
            });
            if is_source_support {
                support_versions.push(MaterializedVersion { version, payload });
            }
        }
        Ok(support_versions)
    }
}
