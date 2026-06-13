use cortex_aql::{AgentView, RetrievalMode};
use cortex_engine::{AqlExplainReport, Database};

use crate::authz;
use crate::responses::{
    AqlCandidateCountsResponse, AqlCellResponse, AqlExecutionOperatorResponse,
    AqlExecutionTraceResponse, AqlExplainFilterResponse, AqlExplainResponse,
    AqlLogicalPlanNodeResponse, AqlLogicalPlanResponse, AqlResponse, RouterError,
};
use crate::router::query_param_decoded;

pub fn handle_aql_shared(
    db: &Database,
    query: &str,
    body: &[u8],
    authenticated_view: Option<&AgentView>,
) -> Result<String, RouterError> {
    let scope = query_param_decoded(query, "scope").map_err(RouterError::BadRequest)?;
    let aql = String::from_utf8_lossy(body);
    let view = authz::read_view_for_scope(&scope, authenticated_view)?;
    if starts_with_explain(&aql) {
        let explain = if starts_with_explain_analyze(&aql) {
            db.explain_analyze_retrieve_aql(&aql, &view)?
        } else {
            db.explain_retrieve_aql(&aql, &view)?
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
        execution_trace: report
            .execution_trace
            .map(|trace| AqlExecutionTraceResponse {
                operators: trace
                    .operators
                    .into_iter()
                    .map(|operator| AqlExecutionOperatorResponse {
                        name: operator.name,
                        input_count: operator.input_count,
                        output_count: operator.output_count,
                        elapsed_nanos: operator.elapsed_nanos,
                    })
                    .collect(),
                total_elapsed_nanos: trace.total_elapsed_nanos,
            }),
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
        RetrievalMode::Semantic => "semantic",
        RetrievalMode::Audit => "audit",
    }
}
