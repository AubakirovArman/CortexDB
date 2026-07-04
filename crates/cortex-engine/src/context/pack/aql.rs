use cortex_aql::{AgentView, BoundPlan};
use cortex_crypto::{blake3_256_domain, hex_lower};
use serde_json::json;

use crate::access_capture::agent_view_digest;
use crate::accountability::AccountabilityDeterminismInput;
use crate::canonical::canonical_json_bytes;
use crate::context::{
    ContextPack, ContextPackOptions, ContextPackReceiptEvidence, ContextPackWithTools,
};
use crate::database::Database;
use crate::determinism_hash::frozen_ranking_weights_identity;
use crate::error::{EngineError, EngineResult};
use crate::exec::PackOp;
use crate::feedback::current_unix_seconds;
use crate::query::{cache::AqlStatementKind, EngineAqlProvider};

const CONTEXT_OPTIONS_DIGEST_DOMAIN: &str = "cortexdb.accountability.context_options_digest.v1";

impl Database {
    /// Compile a `RETRIEVE CONTEXT` AQL statement into a scored ContextPack.
    ///
    /// The same bound retrieve plan drives bitmap filtering, AQL `LIMIT`,
    /// `BUDGET`, `REQUIRE` gates, and the ContextPack citation policy.
    pub fn context_pack_from_aql(
        &self,
        aql: &str,
        view: &AgentView,
        options: ContextPackOptions,
    ) -> EngineResult<ContextPack> {
        self.context_pack_with_receipt_evidence_from_aql(aql, view, options)
            .map(|evidence| evidence.pack)
    }

    pub fn context_pack_with_receipt_evidence_from_aql(
        &self,
        aql: &str,
        view: &AgentView,
        options: ContextPackOptions,
    ) -> EngineResult<ContextPackReceiptEvidence> {
        let (cached, index) = self.bind_aql_cached(aql, view)?;
        if cached.statement_kind != AqlStatementKind::Retrieve {
            return Err(EngineError::InvalidOperation);
        }
        let BoundPlan::Retrieve(plan) = cached.bound_plan else {
            return Err(EngineError::InvalidOperation);
        };
        let provider = EngineAqlProvider::new(index, view);
        let requested_budget = if options.token_budget_tokens == 0 {
            plan.context_policy.budget_tokens
        } else {
            options.token_budget_tokens
        };
        let budget = view.effective_budget(requested_budget);
        let citations_required = options.require_citations || plan.context_policy.require_citations;
        let report = self.retrieve_cells_with_execution_trace(&plan, &provider)?;
        let feedback_scores = self.feedback_scores_for_cells_at(
            report.cells.iter().map(|cell| cell.cell_id),
            current_unix_seconds(),
        );
        let cells = crate::context::scoring::order_by_feedback(report.cells, &feedback_scores);
        let receipt_cells = cells.clone();
        let pack = PackOp::execute(
            cells,
            budget,
            citations_required,
            &options,
            aql,
            &feedback_scores,
            Some(view),
        )
        .pack;
        let frozen_weights = frozen_ranking_weights_identity();
        let determinism_input = AccountabilityDeterminismInput {
            query: aql.to_owned(),
            agent_view_digest: Some(agent_view_digest(view)),
            context_options_digest: Some(context_options_digest(
                requested_budget,
                budget,
                citations_required,
                &options,
            )),
            bitmap_program_digest: None,
            frozen_weights_version: frozen_weights.version,
            frozen_weights_hash: frozen_weights.artifact_hash,
            ann_serving_epoch: self.current_ann_serving_epoch(),
            embedding_ref: self.current_receipt_embedding_ref(),
        };
        Ok(ContextPackReceiptEvidence::new(
            pack,
            receipt_cells,
            report.captured_access_denials,
            determinism_input,
        ))
    }

    pub fn context_pack_with_tool_recommendations_from_aql(
        &self,
        aql: &str,
        view: &AgentView,
        options: ContextPackOptions,
        tool_limit: usize,
    ) -> EngineResult<ContextPackWithTools> {
        let pack = self.context_pack_from_aql(aql, view, options)?;
        let (cached, _) = self.bind_aql_cached(aql, view)?;
        let BoundPlan::Retrieve(plan) = cached.bound_plan else {
            return Err(EngineError::InvalidOperation);
        };
        let tool_recommendations = self.recommend_tools_for_task(view, &plan.task, tool_limit);
        Ok(ContextPackWithTools {
            pack,
            tool_recommendations,
        })
    }
}

fn context_options_digest(
    requested_budget_tokens: u32,
    effective_budget_tokens: u32,
    citations_required: bool,
    options: &ContextPackOptions,
) -> String {
    let value = json!({
        "schema_version": "cortexdb.accountability.context_options.v1",
        "requested_budget_tokens": requested_budget_tokens,
        "effective_budget_tokens": effective_budget_tokens,
        "citations_required": citations_required,
        "options": {
            "token_budget_tokens": options.token_budget_tokens,
            "require_citations": options.require_citations,
            "reduce_redundancy": options.reduce_redundancy,
            "redundancy_threshold_q16": options.redundancy_threshold_q16,
            "citation_overhead_tokens": options.citation_overhead_tokens,
            "token_profile": options.token_profile.as_str(),
            "large_cell_policy": options.large_cell_policy.as_str(),
            "span_level_packing": options.span_level_packing,
            "span_context_lines": options.span_context_lines,
            "optimize_value_per_token": options.optimize_value_per_token,
        },
    });
    hex_lower(&blake3_256_domain(
        CONTEXT_OPTIONS_DIGEST_DOMAIN,
        &canonical_json_bytes(&value),
    ))
}
