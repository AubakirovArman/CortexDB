use std::collections::BTreeSet;

use cortex_aql::{AgentView, BoundPlan};

use super::VerificationReport;
use crate::access_capture::{agent_view_digest, captured_allowed_access_decision};
use crate::accountability::AccountabilityDeterminismInput;
use crate::context::{ContextPack, ContextPackOptions, ContextPackReceiptEvidence};
use crate::database::Database;
use crate::database::{CapturedAccessDenialSet, RetrievedCell};
use crate::determinism_hash::frozen_ranking_weights_identity;
use crate::error::{EngineError, EngineResult};
use crate::query::cache::AqlStatementKind;

impl Database {
    /// Execute a `VERIFY FACT` AQL statement against stored evidence.
    ///
    /// See `tests/verification_tests.rs` for end-to-end examples.
    pub fn verify_fact_aql(&self, aql: &str, view: &AgentView) -> EngineResult<VerificationReport> {
        let cached = self.bind_verify_fact_cached(aql, view)?;
        if cached.statement_kind != AqlStatementKind::VerifyFact {
            return Err(EngineError::InvalidOperation);
        }
        let BoundPlan::VerifyFact(plan) = cached.bound_plan else {
            return Err(EngineError::InvalidOperation);
        };
        self.execute_verify_fact_plan(*plan, view)
            .map(|execution| execution.report)
    }

    pub fn verify_fact_with_receipt_evidence_aql(
        &self,
        aql: &str,
        view: &AgentView,
    ) -> EngineResult<(VerificationReport, ContextPackReceiptEvidence)> {
        let cached = self.bind_verify_fact_cached(aql, view)?;
        if cached.statement_kind != AqlStatementKind::VerifyFact {
            return Err(EngineError::InvalidOperation);
        }
        let BoundPlan::VerifyFact(plan) = cached.bound_plan else {
            return Err(EngineError::InvalidOperation);
        };
        let report = self.execute_verify_fact_plan(*plan, view)?.report;
        let receipt_evidence = self.verification_receipt_evidence(&report, aql, view)?;
        Ok((report, receipt_evidence))
    }

    fn verification_receipt_evidence(
        &self,
        report: &VerificationReport,
        aql: &str,
        view: &AgentView,
    ) -> EngineResult<ContextPackReceiptEvidence> {
        let mut cell_ids = BTreeSet::new();
        cell_ids.extend(report.evidence.iter().map(|item| item.cell_id));
        cell_ids.extend(
            report
                .contradicting_evidence
                .iter()
                .map(|item| item.cell_id),
        );
        cell_ids.extend(report.numeric_conflicts.iter().map(|item| item.cell_id));

        let mut cells = Vec::new();
        for cell_id in cell_ids {
            let (payload, descriptor) = self
                .get_latest_cell_with_descriptor(cell_id)
                .ok_or(EngineError::InvalidOperation)?;
            let captured_access_decision =
                Some(captured_allowed_access_decision(cell_id, &descriptor, view));
            cells.push(RetrievedCell {
                cell_id,
                payload,
                descriptor,
                captured_access_decision,
            });
        }

        let options = ContextPackOptions {
            token_budget_tokens: u32::MAX / 4,
            ..ContextPackOptions::default()
        };
        let pack = ContextPack::from_retrieved_with_feedback_options_and_view(
            cells.clone(),
            options.token_budget_tokens,
            false,
            &options,
            &report.fact,
            &Default::default(),
            Some(view),
        );
        let frozen_weights = frozen_ranking_weights_identity();
        let determinism_input = AccountabilityDeterminismInput {
            query: aql.to_owned(),
            agent_view_digest: Some(agent_view_digest(view)),
            context_options_digest: None,
            bitmap_program_digest: None,
            frozen_weights_version: frozen_weights.version,
            frozen_weights_hash: frozen_weights.artifact_hash,
            ann_serving_epoch: self.current_ann_serving_epoch(),
        };
        Ok(ContextPackReceiptEvidence::new(
            pack,
            cells,
            CapturedAccessDenialSet::default(),
            determinism_input,
        ))
    }
}
