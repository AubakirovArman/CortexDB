use cortex_aql::RetrievalMode;
use cortex_engine::{AqlExplainReport, RetrievedCell};
use serde_json::to_string;

use crate::cli_json_types::{
    AqlCandidateCountsResponse, AqlCellResponse, AqlExecutionOperatorResponse,
    AqlExecutionTraceResponse, AqlExplainFilterResponse, AqlExplainResponse,
    AqlLogicalPlanNodeResponse, AqlLogicalPlanResponse, AqlResponse,
};

pub(crate) fn aql_to_json(cells: &[RetrievedCell]) -> String {
    serialize_or_error(&AqlResponse {
        cells: cells
            .iter()
            .map(|c| AqlCellResponse {
                cell_id: c.cell_id.0,
                payload: String::from_utf8_lossy(&c.payload).into_owned(),
            })
            .collect(),
        explain: None,
    })
}

pub(crate) fn aql_explain_to_json(report: AqlExplainReport) -> String {
    serialize_or_error(&AqlResponse {
        cells: Vec::new(),
        explain: Some(AqlExplainResponse {
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
        }),
    })
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

fn serialize_or_error<T: serde::Serialize>(value: &T) -> String {
    to_string(value).unwrap_or_else(|e| {
        to_string(&crate::cli_json_types::ErrorResponse {
            code: "internal".to_owned(),
            error: "internal_error".to_owned(),
            message: e.to_string(),
        })
        .unwrap_or_else(|_| {
            "{\"code\":\"internal\",\"error\":\"internal_error\",\"message\":\"serialization failed\"}".to_owned()
        })
    })
}
