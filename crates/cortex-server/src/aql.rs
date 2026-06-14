use cortex_aql::{AgentView, RetrievalMode};
use cortex_engine::{AqlExplainReport, Database};

use crate::authz;
use crate::responses::{
    AqlCandidateCountsResponse, AqlCellResponse, AqlExecutionOperatorResponse,
    AqlExecutionTraceResponse, AqlExplainFilterResponse, AqlExplainResponse,
    AqlLogicalPlanNodeResponse, AqlLogicalPlanResponse, AqlResponse, RouterError,
};
use crate::router::{query_param_decoded, query_param_opt_decoded};

pub fn handle_aql_shared(
    db: &Database,
    query: &str,
    body: &[u8],
    authenticated_view: Option<&AgentView>,
) -> Result<String, RouterError> {
    let scope = query_param_decoded(query, "scope").map_err(RouterError::BadRequest)?;
    let aql = String::from_utf8_lossy(body);
    let view = authz::read_view_for_scope(&scope, authenticated_view)?;
    let query_explain_mode = query_explain_mode(query)?;
    let body_explain_mode = explain_mode_from_statement(&aql);
    if let Some(mode) = query_explain_mode.or(body_explain_mode) {
        let statement = explain_statement(&aql, mode);
        let explain = if mode == AqlExplainMode::Analyze {
            db.explain_analyze_retrieve_aql(&statement, &view)?
        } else {
            db.explain_retrieve_aql(&statement, &view)?
        };
        let response = AqlResponse {
            cells: Vec::new(),
            explain: Some(explain_response(explain)),
        };
        return Ok(serde_json::to_string(&response)?);
    }
    let cells = db.retrieve_aql(&aql, &view)?;
    let response = AqlResponse {
        cells: cells
            .iter()
            .map(|cell| AqlCellResponse {
                cell_id: cell.cell_id.0,
                payload: String::from_utf8_lossy(&cell.payload).into_owned(),
            })
            .collect(),
        explain: None,
    };
    Ok(serde_json::to_string(&response)?)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum AqlExplainMode {
    Plan,
    Analyze,
}

fn query_explain_mode(query: &str) -> Result<Option<AqlExplainMode>, RouterError> {
    let Some(value) = query_param_opt_decoded(query, "explain") else {
        return Ok(None);
    };
    match value.to_ascii_lowercase().as_str() {
        "plan" | "true" | "1" => Ok(Some(AqlExplainMode::Plan)),
        "analyze" => Ok(Some(AqlExplainMode::Analyze)),
        "false" | "0" => Ok(None),
        other => Err(RouterError::BadRequest(format!(
            "unsupported explain mode '{other}' (expected plan or analyze)"
        ))),
    }
}

fn explain_mode_from_statement(aql: &str) -> Option<AqlExplainMode> {
    if starts_with_explain_analyze(aql) {
        Some(AqlExplainMode::Analyze)
    } else if starts_with_explain(aql) {
        Some(AqlExplainMode::Plan)
    } else {
        None
    }
}

fn explain_statement(aql: &str, mode: AqlExplainMode) -> String {
    let statement = statement_without_explain_prefix(aql);
    match mode {
        AqlExplainMode::Plan => format!("EXPLAIN {statement}"),
        AqlExplainMode::Analyze => format!("EXPLAIN ANALYZE {statement}"),
    }
}

fn statement_without_explain_prefix(aql: &str) -> &str {
    let trimmed = aql.trim_start();
    if starts_with_explain_analyze(trimmed) {
        trimmed[15..].trim_start()
    } else if starts_with_explain(trimmed) {
        trimmed[7..].trim_start()
    } else {
        trimmed
    }
}

fn starts_with_explain_analyze(aql: &str) -> bool {
    let trimmed = aql.trim_start();
    trimmed
        .get(..15)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("EXPLAIN ANALYZE"))
        && trimmed
            .chars()
            .nth(15)
            .is_none_or(|ch| ch.is_ascii_whitespace())
}

fn starts_with_explain(aql: &str) -> bool {
    let trimmed = aql.trim_start();
    let Some(prefix) = trimmed.get(..7) else {
        return false;
    };
    if !prefix.eq_ignore_ascii_case("EXPLAIN") {
        return false;
    }
    trimmed
        .chars()
        .nth(7)
        .is_none_or(|ch| ch.is_ascii_whitespace())
}

fn explain_response(report: AqlExplainReport) -> AqlExplainResponse {
    let execution_trace = report
        .execution_trace
        .as_ref()
        .map(|trace| AqlExecutionTraceResponse {
            operators: trace
                .operators
                .iter()
                .map(|operator| AqlExecutionOperatorResponse {
                    name: operator.name.clone(),
                    input_count: operator.input_count,
                    output_count: operator.output_count,
                    actual_input_count: operator.input_count,
                    actual_output_count: operator.output_count,
                    estimated_output_count: estimated_operator_output_count(operator, &report),
                    elapsed_nanos: operator.elapsed_nanos,
                })
                .collect(),
            total_elapsed_nanos: trace.total_elapsed_nanos,
        });
    AqlExplainResponse {
        task: report.task,
        brain_id: report.brain_id.0,
        selected_mode: retrieval_mode_name(report.selected_mode).to_owned(),
        logical_plan: logical_plan_response(report.logical_plan),
        policy_rewritten_plan: logical_plan_response(report.policy_rewritten_plan),
        bitmap_plan: report.bitmap_plan,
        bitmap_ops: report.bitmap_ops,
        filters: report
            .filters
            .into_iter()
            .map(|filter| AqlExplainFilterResponse {
                kind: filter.kind,
                expression: filter.expression,
            })
            .collect(),
        cost_model: Some(cost_model_response(report.cost_model)),
        candidate_counts: AqlCandidateCountsResponse {
            universe: report.candidate_counts.universe,
            agent_allowed: report.candidate_counts.agent_allowed,
            live: report.candidate_counts.live,
            estimated_after_bitmap: report.candidate_counts.estimated_after_bitmap,
            after_bitmap: report.candidate_counts.after_bitmap,
            after_quality: report.candidate_counts.after_quality,
            returned_limit: report.candidate_counts.returned_limit,
        },
        candidate_limit: report.candidate_limit,
        budget_tokens: report.budget_tokens,
        citations_required: report.citations_required,
        execution_trace,
    }
}

fn estimated_operator_output_count(
    operator: &cortex_engine::PhysicalOperatorTrace,
    report: &AqlExplainReport,
) -> Option<usize> {
    match operator.name.as_str() {
        "BitmapIndexScan" => report.candidate_counts.estimated_after_bitmap,
        "PermissionFilter" => Some(report.candidate_counts.agent_allowed),
        "MemoryLifecycleFilter" => Some(report.candidate_counts.live),
        "QualityFilter" => Some(report.candidate_counts.after_quality),
        "LimitOp" | "PackOp" => Some(report.candidate_counts.returned_limit),
        _ => None,
    }
}

fn cost_model_response(
    decision: cortex_engine::CostModelDecision,
) -> crate::responses::AqlCostModelResponse {
    crate::responses::AqlCostModelResponse {
        selected_path: decision.selected_path.as_str().to_owned(),
        reason: decision.reason,
        estimated_live_rows: decision.estimated_live_rows,
        estimated_after_bitmap: decision.estimated_after_bitmap,
        recommended_candidate_limit: decision.recommended_candidate_limit,
        has_query_vector: decision.has_query_vector,
        rarest_term: decision
            .rarest_term
            .map(|term| crate::responses::AqlCostModelTermResponse {
                term: term.term,
                document_frequency: term.document_frequency,
            }),
        estimates: decision
            .estimates
            .into_iter()
            .map(|estimate| crate::responses::AqlCostModelEstimateResponse {
                path: estimate.path.as_str().to_owned(),
                cost_units: estimate.cost_units,
            })
            .collect(),
    }
}

fn logical_plan_response(report: cortex_engine::LogicalPlanReport) -> AqlLogicalPlanResponse {
    AqlLogicalPlanResponse {
        nodes: report
            .nodes
            .into_iter()
            .map(|node| AqlLogicalPlanNodeResponse {
                id: node.id,
                kind: node.kind,
                detail: node.detail,
                permission_predicate: node.permission_predicate,
            })
            .collect(),
        policy_complete: report.policy_complete,
    }
}

fn retrieval_mode_name(mode: RetrievalMode) -> &'static str {
    match mode {
        RetrievalMode::Fast => "fast",
        RetrievalMode::Balanced => "balanced",
        RetrievalMode::Hybrid => "hybrid",
        RetrievalMode::Semantic => "semantic",
        RetrievalMode::Audit => "audit",
    }
}
