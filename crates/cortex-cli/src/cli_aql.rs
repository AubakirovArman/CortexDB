use cortex_engine::AqlExplainReport;

use crate::cli_aql_json::{aql_explain_to_json, aql_to_json};
use crate::cli_ops::{fmt_engine_error, open_database};
use crate::context::{format_retrieved_cells, view_for_scope};

pub fn aql(path: &str, scope: &str, aql: &str, json: bool) -> Result<String, String> {
    let db = open_database(path, false)?;
    if starts_with_explain(aql) {
        let report = db
            .explain_retrieve_aql(aql, &view_for_scope(scope))
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
    format!(
        "aql_explain task={} mode={:?} brain_id={} candidate_limit={} budget_tokens={} citations_required={}\nlogical_plan_policy_complete={} policy_rewritten_plan_policy_complete={}\ncounts universe={} agent_allowed={} live={} after_bitmap={} after_quality={} returned_limit={}\nfilters={}\n{}",
        report.task,
        report.selected_mode,
        report.brain_id.0,
        report.candidate_limit,
        report.budget_tokens,
        report.citations_required,
        report.logical_plan.policy_complete,
        report.policy_rewritten_plan.policy_complete,
        report.candidate_counts.universe,
        report.candidate_counts.agent_allowed,
        report.candidate_counts.live,
        report.candidate_counts.after_bitmap,
        report.candidate_counts.after_quality,
        report.candidate_counts.returned_limit,
        filters,
        report.bitmap_plan
    )
}
