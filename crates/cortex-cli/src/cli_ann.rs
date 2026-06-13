use cortex_engine::{
    evaluate_hnsw_no_fallback_rollout, parse_vector_literal, AnnEvaluationReport, AnnSearchPolicy,
    HnswNoFallbackDecision, HnswNoFallbackRolloutPolicy, SearchLimit,
};

use crate::cli_json::{ann_evaluation_to_json, CliAnnEvaluationJsonInput};
use crate::context::view_for_scope;

pub struct SearchVectorEvalOptions<'a> {
    pub path: &'a str,
    pub scope: &'a str,
    pub vector: &'a str,
    pub json: bool,
    pub policy: Option<AnnSearchPolicy>,
    pub rollout_policy: Option<HnswNoFallbackRolloutPolicy>,
    pub use_no_fallback_profile: bool,
    pub experimental_hnsw: bool,
}

pub fn search_vector_eval(options: SearchVectorEvalOptions<'_>) -> Result<String, String> {
    let vector = parse_vector_literal(options.vector)?;
    let db = crate::cli_ops::open_database(options.path, options.experimental_hnsw)?;
    let rollout_policy = crate::cli_ops::resolve_no_fallback_profile(
        &db,
        options.rollout_policy,
        options.use_no_fallback_profile,
    )?;
    let search_policy = options.policy.unwrap_or_default();
    let report = db
        .evaluate_vector_ann_with_policy(
            &vector,
            &view_for_scope(options.scope),
            SearchLimit(20),
            search_policy,
        )
        .map_err(|error| error.to_string())?;
    if options.json {
        return Ok(match report {
            Some(report) => {
                let decision = rollout_policy.map(|rollout_policy| {
                    no_fallback_decision_to_json(&evaluate_hnsw_no_fallback_rollout(
                        rollout_policy,
                        search_policy,
                        &report.search,
                    ))
                });
                ann_evaluation_to_json(CliAnnEvaluationJsonInput {
                    available: true,
                    reason: None,
                    report: Some(to_ann_search_report(&report)),
                    no_fallback_decision: decision,
                    exact_top_k: report.exact_top_k.clone(),
                    ann_top_k: report.ann_top_k.clone(),
                    overlap_count: report.overlap_count,
                    recall_q16: report.recall_q16,
                })
            }
            None => ann_evaluation_to_json(CliAnnEvaluationJsonInput {
                available: false,
                reason: Some("requires_persisted_checkpoint_without_wal_tail".to_owned()),
                report: None,
                no_fallback_decision: None,
                exact_top_k: Vec::new(),
                ann_top_k: Vec::new(),
                overlap_count: 0,
                recall_q16: 0,
            }),
        });
    }
    Ok(match report {
        Some(report) => format_ann_evaluation(&report, rollout_policy, search_policy),
        None => {
            "ann_evaluation available=false reason=requires_persisted_checkpoint_without_wal_tail"
                .to_owned()
        }
    })
}

fn format_ann_evaluation(
    report: &AnnEvaluationReport,
    rollout_policy: Option<HnswNoFallbackRolloutPolicy>,
    search_policy: AnnSearchPolicy,
) -> String {
    let violations = if report.search.slo_violations.is_empty() {
        "none".to_owned()
    } else {
        report
            .search
            .slo_violations
            .iter()
            .map(|violation| violation.as_str())
            .collect::<Vec<_>>()
            .join(",")
    };
    let mut line = format!(
        "ann_evaluation available=true path={} fallback_reason={} fallback_performed={} recall_q16={} min_recall_q16={} require_slo={} production_safe={} slo_violations={} overlap_count={} exact_top_k={:?} ann_top_k={:?}",
        report.search.path.as_str(),
        report
            .search
            .fallback_reason
            .map(|reason| reason.as_str())
            .unwrap_or("null"),
        report.search.fallback_performed,
        report.recall_q16,
        report
            .search
            .min_recall_q16
            .map(|value| value.to_string())
            .unwrap_or_else(|| "null".to_owned()),
        report.search.require_slo,
        report.search.production_safe,
        violations,
        report.overlap_count,
        report.exact_top_k,
        report.ann_top_k
    );
    if let Some(rollout_policy) = rollout_policy {
        let decision =
            evaluate_hnsw_no_fallback_rollout(rollout_policy, search_policy, &report.search);
        line.push('\n');
        line.push_str(&format_no_fallback_decision(&decision));
    }
    line
}

fn to_ann_search_report(
    report: &AnnEvaluationReport,
) -> crate::cli_json_types::CliAnnSearchReportResponse {
    crate::cli_json_types::CliAnnSearchReportResponse {
        path: report.search.path.as_str().to_owned(),
        fallback_reason: report
            .search
            .fallback_reason
            .map(|reason| reason.as_str().to_owned()),
        requested_limit: report.search.requested_limit,
        allowed_candidates: report.search.allowed_candidates,
        graph_nodes: report.search.graph_nodes,
        returned_candidates: report.search.returned_candidates,
        visited_candidates: report.search.visited_candidates,
        max_visited_candidates: report.search.max_visited_candidates,
        recall_q16: report.search.recall_q16,
        min_recall_q16: report.search.min_recall_q16,
        hnsw_max_neighbors: report.search.hnsw_max_neighbors,
        hnsw_ef_search: report.search.hnsw_ef_search,
        hnsw_ef_construction: report.search.hnsw_ef_construction,
        hnsw_layer_count: report.search.hnsw_layer_count,
        upper_graph_edges: report.search.upper_graph_edges,
        fallback_performed: report.search.fallback_performed,
        require_slo: report.search.require_slo,
        production_safe: report.search.production_safe,
        slo_violations: report
            .search
            .slo_violations
            .iter()
            .map(|violation| violation.as_str().to_owned())
            .collect(),
    }
}

pub(crate) fn parse_no_fallback_rollout_policy(
    rollout: bool,
    min_recall: Option<String>,
) -> Result<Option<HnswNoFallbackRolloutPolicy>, String> {
    if !rollout && min_recall.is_none() {
        return Ok(None);
    }
    if !rollout {
        return Err("no_fallback_min_recall requires --no-fallback-rollout".to_owned());
    }
    let enabled_policy = HnswNoFallbackRolloutPolicy::enabled();
    let policy = HnswNoFallbackRolloutPolicy {
        min_recall_q16: match min_recall {
            Some(value) => parse_min_recall_q16(&value)?,
            None => enabled_policy.min_recall_q16,
        },
        ..enabled_policy
    };
    Ok(Some(policy))
}

pub(crate) fn parse_no_fallback_profile(
    enabled: String,
    min_recall: Option<String>,
    require_upper_layers: String,
) -> Result<HnswNoFallbackRolloutPolicy, String> {
    let enabled_policy = HnswNoFallbackRolloutPolicy::enabled();
    Ok(HnswNoFallbackRolloutPolicy {
        rollout_enabled: parse_bool_value("enabled", &enabled)?,
        min_recall_q16: match min_recall {
            Some(value) => parse_min_recall_q16(&value)?,
            None => enabled_policy.min_recall_q16,
        },
        require_upper_layers: parse_bool_value("require_upper_layers", &require_upper_layers)?,
    })
}

pub(crate) fn parse_ann_policy(
    fallback: Option<String>,
    fallback_scan_cap: Option<usize>,
    min_recall: Option<String>,
    max_visited_candidates: Option<usize>,
    require_slo: bool,
) -> Result<AnnSearchPolicy, String> {
    let default_policy = AnnSearchPolicy::default();
    let fallback = parse_option_bool("fallback", fallback)?.unwrap_or(default_policy.fallback);
    let min_recall_q16 = match min_recall {
        Some(value) => Some(parse_min_recall_q16(&value)?),
        None => default_policy.min_recall_q16,
    };
    Ok(AnnSearchPolicy {
        min_recall_q16,
        fallback,
        fallback_scan_cap,
        max_visited_candidates,
        require_slo,
    })
}

pub(crate) fn format_no_fallback_decision(decision: &HnswNoFallbackDecision) -> String {
    let reasons = if decision.reasons.is_empty() {
        "none".to_owned()
    } else {
        decision
            .reasons
            .iter()
            .map(|reason| reason.as_str())
            .collect::<Vec<_>>()
            .join(",")
    };
    format!(
        "no_fallback_allowed={} no_fallback_reasons={}",
        decision.allowed, reasons
    )
}

pub(crate) fn no_fallback_decision_to_json(
    decision: &HnswNoFallbackDecision,
) -> crate::cli_json_types::CliNoFallbackDecisionResponse {
    crate::cli_json_types::CliNoFallbackDecisionResponse {
        allowed: decision.allowed,
        reasons: decision
            .reasons
            .iter()
            .map(|reason| reason.as_str().to_owned())
            .collect(),
    }
}

fn parse_option_bool(name: &str, value: Option<String>) -> Result<Option<bool>, String> {
    value
        .map(|value| parse_bool_value(name, &value))
        .transpose()
}

pub(crate) fn parse_bool_value(name: &str, value: &str) -> Result<bool, String> {
    match value.to_ascii_lowercase().as_str() {
        "true" | "1" | "yes" | "on" => Ok(true),
        "false" | "0" | "no" | "off" => Ok(false),
        _ => Err(format!("{name} must be true/false")),
    }
}

fn parse_min_recall_q16(value: &str) -> Result<u16, String> {
    let normalized = value.trim();
    let ratio = if normalized.ends_with('%') {
        let percent =
            parse_percent_without_unit(&normalized[..normalized.len().saturating_sub(1)])?;
        percent / 100.0
    } else {
        let number = normalized.parse::<f64>().map_err(|_| {
            "min_recall must be a decimal fraction, percentage, or integer q16".to_owned()
        })?;
        if number > 1.0 && number <= 100.0 {
            number / 100.0
        } else if number > 100.0 && number <= f64::from(u16::MAX) {
            number / f64::from(u16::MAX)
        } else {
            number
        }
    };

    if !(0.0..=1.0).contains(&ratio) {
        return Err("min_recall must be in [0.0, 1.0] or [0,100]%".to_owned());
    }

    let value = (ratio * f64::from(u16::MAX)).floor();
    Ok(value as u16)
}

fn parse_percent_without_unit(value: &str) -> Result<f64, String> {
    value
        .trim()
        .parse::<f64>()
        .map_err(|_| "min_recall must be percentage value".to_owned())
}

#[cfg(test)]
mod tests;
