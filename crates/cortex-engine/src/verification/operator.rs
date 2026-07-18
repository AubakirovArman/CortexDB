use std::time::Instant;

use cortex_aql::{AgentView, BoundVerifyFactPlan};

use super::evidence::{
    contradiction_for_version, evidence_for_version, sort_evidence, verification_confidence_q16,
    verification_status,
};
use super::graph::{
    add_graph_relation_contradictions_from_versions, enrich_evidence_from_source_support_edges,
};
use super::guards::{
    citation_guard, numeric_mismatch_conflict, numeric_mismatch_guard, stale_fact_guard,
    stale_fact_guard_with_reason,
};
use super::temporal::extract_temporal_query_range;
use super::VerificationReport;
use crate::database::Database;
use crate::error::EngineResult;
use crate::exec::PhysicalOperatorTrace;

mod candidates;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct VerificationExecutionReport {
    pub(crate) report: VerificationReport,
    pub(crate) operators: Vec<PhysicalOperatorTrace>,
    pub(crate) total_elapsed_nanos: u64,
}

impl Database {
    pub(crate) fn execute_verify_fact_plan(
        &self,
        plan: BoundVerifyFactPlan,
        view: &AgentView,
    ) -> EngineResult<VerificationExecutionReport> {
        let total_started = Instant::now();
        let mut operators = Vec::new();
        let mut evidence = Vec::new();
        let mut contradicting_evidence = Vec::new();
        let mut guards = Vec::new();
        let mut numeric_conflicts = Vec::new();
        let pin = self.pin_read_txn();
        let txn = pin.read_txn();

        let started = Instant::now();
        let temporal_query = extract_temporal_query_range(&plan.fact);
        let indexed_stale_cell_ids = temporal_query
            .map(|query| {
                self.derived_stores
                    .temporal_validity_store
                    .stale_cell_ids_for_range(query)
            })
            .unwrap_or_default();
        if temporal_query.is_some() {
            operators.push(trace(
                "VerificationTemporalIndexLookup",
                0,
                indexed_stale_cell_ids.len(),
                started,
            ));
        }

        let started = Instant::now();
        let indexed_numeric_cell_ids = self
            .derived_stores
            .fact_claim_store
            .indexed_cell_ids_for_fact(&plan.fact, view);
        let mut candidate_versions = if indexed_numeric_cell_ids.is_empty() {
            self.verification_candidate_versions(&plan.fact, txn)?
        } else {
            indexed_numeric_cell_ids
                .into_iter()
                .filter_map(|cell_id| self.memtable.read(txn, cell_id))
                .collect::<Vec<_>>()
        };
        candidate_versions.sort_by_key(|version| version.cell_id);
        candidate_versions.dedup_by_key(|version| version.cell_id);
        candidate_versions.truncate(plan.max_candidates as usize);
        let candidate_count = candidate_versions.len();
        operators.push(trace(
            "VerificationCandidateScan",
            0,
            candidate_count,
            started,
        ));

        let started = Instant::now();
        let candidate_versions =
            self.filter_verification_candidate_permissions(candidate_versions, view);
        operators.push(trace(
            "VerificationPermissionFilter",
            candidate_count,
            candidate_versions.len(),
            started,
        ));

        let started = Instant::now();
        let candidate_versions = self.materialize_verification_versions(candidate_versions)?;
        operators.push(trace(
            "VerificationMaterializeOp",
            candidate_count,
            candidate_versions.len(),
            started,
        ));

        let started = Instant::now();
        for candidate in &candidate_versions {
            let version = candidate.version;
            let payload = candidate.payload.as_slice();
            let indexed_stale_reason = temporal_query.and_then(|query| {
                indexed_stale_cell_ids.contains(&version.cell_id).then(|| {
                    self.derived_stores
                        .temporal_validity_store
                        .stale_reason_for_cell(version.cell_id, query)
                })?
            });
            let stale_guard = indexed_stale_reason
                .and_then(|reason| stale_fact_guard_with_reason(&plan.fact, version, view, reason))
                .or_else(|| stale_fact_guard(&plan.fact, version, view));
            if let Some(guard) = stale_guard {
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
        self.derived_stores.fact_claim_store.add_verify_matches(
            &plan.fact,
            view,
            &mut evidence,
            &mut contradicting_evidence,
            &mut numeric_conflicts,
        );
        operators.push(trace(
            "VerifyOp",
            candidate_versions.len(),
            evidence.len() + contradicting_evidence.len(),
            started,
        ));

        let started = Instant::now();
        let support_input_count = evidence.len();
        let support_output_count =
            enrich_evidence_from_source_support_edges(self, view, &mut evidence);
        operators.push(trace(
            "SourceSupportExpandOp",
            support_input_count,
            support_output_count,
            started,
        ));

        let started = Instant::now();
        sort_evidence(&mut evidence);
        sort_evidence(&mut contradicting_evidence);
        evidence.truncate(plan.max_evidence as usize);
        contradicting_evidence.truncate(plan.max_evidence as usize);
        let status = verification_status(!evidence.is_empty(), !contradicting_evidence.is_empty());
        let confidence_q16 =
            verification_confidence_q16(status, &evidence, &contradicting_evidence);
        let report = VerificationReport {
            fact: plan.fact,
            status,
            confidence_q16,
            evidence,
            contradicting_evidence,
            guards,
            numeric_conflicts,
        };
        operators.push(trace("VerdictAggregateOp", candidate_count, 1, started));

        Ok(VerificationExecutionReport {
            report,
            operators,
            total_elapsed_nanos: elapsed_nanos(total_started),
        })
    }
}

fn trace(
    name: &str,
    input_count: usize,
    output_count: usize,
    started: Instant,
) -> PhysicalOperatorTrace {
    PhysicalOperatorTrace {
        name: name.to_owned(),
        input_count,
        output_count,
        elapsed_nanos: elapsed_nanos(started),
    }
}

fn elapsed_nanos(started: Instant) -> u64 {
    u64::try_from(started.elapsed().as_nanos()).unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests;
