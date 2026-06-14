use cortex_engine::AqlExplainReport;

use crate::cli_aql_json::{aql_explain_to_json, aql_to_json};
use crate::cli_ops::{fmt_engine_error, open_database};
use crate::context::{format_retrieved_cells, view_for_scope};

pub fn aql(
    path: &str,
    scope: &str,
    aql: &str,
    json: bool,
    explain: Option<&str>,
) -> Result<String, String> {
    let db = open_database(path, false)?;
    if let Some(mode) = explain_mode(aql, explain)? {
        let statement = explain_statement(aql, mode);
        let report = if mode == AqlExplainMode::Analyze {
            db.explain_analyze_retrieve_aql(&statement, &view_for_scope(scope))
        } else {
            db.explain_retrieve_aql(&statement, &view_for_scope(scope))
        }
        .map_err(fmt_engine_error)?;
        if json {
            return Ok(aql_explain_to_json(report));
        }
        return Ok(format_aql_explain(&report));
    }
    let cells = db
        .retrieve_aql(aql, &view_for_scope(scope))
        .map_err(fmt_engine_error)?;
    if json {
        Ok(aql_to_json(&cells))
    } else {
        Ok(format_retrieved_cells(&cells))
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum AqlExplainMode {
    Plan,
    Analyze,
}

fn explain_mode(aql: &str, explain: Option<&str>) -> Result<Option<AqlExplainMode>, String> {
    if let Some(mode) = explain {
        return match mode {
            "plan" => Ok(Some(AqlExplainMode::Plan)),
            "analyze" => Ok(Some(AqlExplainMode::Analyze)),
            other => Err(format!(
                "unsupported explain mode '{other}' (expected plan or analyze)"
            )),
        };
    }
    if starts_with_explain_analyze(aql) {
        Ok(Some(AqlExplainMode::Analyze))
    } else if starts_with_explain(aql) {
        Ok(Some(AqlExplainMode::Plan))
    } else {
        Ok(None)
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

fn format_aql_explain(report: &AqlExplainReport) -> String {
    let filters = report
        .filters
        .iter()
        .map(|filter| format!("{}={}", filter.kind, filter.expression))
        .collect::<Vec<_>>()
        .join(", ");
    let mut output = format!(
        "aql_explain task={} mode={:?} brain_id={} candidate_limit={} budget_tokens={} citations_required={}\nlogical_plan_policy_complete={} policy_rewritten_plan_policy_complete={} execution_trace_operators={}\ncost_model selected_path={} reason={} recommended_candidate_limit={} has_query_vector={}\ncounts universe={} agent_allowed={} live={} estimated_after_bitmap={} after_bitmap={} after_quality={} returned_limit={}\nfilters={}\n{}",
        report.task,
        report.selected_mode,
        report.brain_id.0,
        report.candidate_limit,
        report.budget_tokens,
        report.citations_required,
        report.logical_plan.policy_complete,
        report.policy_rewritten_plan.policy_complete,
        report
            .execution_trace
            .as_ref()
            .map(|trace| trace.operators.len())
            .unwrap_or_default(),
        report.cost_model.selected_path.as_str(),
        report.cost_model.reason,
        report.cost_model.recommended_candidate_limit,
        report.cost_model.has_query_vector,
        report.candidate_counts.universe,
        report.candidate_counts.agent_allowed,
        report.candidate_counts.live,
        report
            .candidate_counts
            .estimated_after_bitmap
            .map(|count| count.to_string())
            .unwrap_or_else(|| "n/a".to_owned()),
        report.candidate_counts.after_bitmap,
        report.candidate_counts.after_quality,
        report.candidate_counts.returned_limit,
        filters,
        report.bitmap_plan
    );
    if let Some(trace) = &report.execution_trace {
        output.push_str(&format!(
            "\nexecution_trace total_elapsed_nanos={}",
            trace.total_elapsed_nanos
        ));
        for operator in &trace.operators {
            output.push_str(&format!(
                "\noperator name={} actual_input_count={} actual_output_count={} estimated_output_count={} elapsed_nanos={}",
                operator.name,
                operator.input_count,
                operator.output_count,
                estimated_operator_output_count(operator, report)
                    .map(|count| count.to_string())
                    .unwrap_or_else(|| "null".to_owned()),
                operator.elapsed_nanos
            ));
        }
    }
    output
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
