use cortex_engine::{
    parse_vector_literal, AnnEvaluationReport, AnnSearchPolicy, Database, SearchLimit,
};

use crate::cli_json::ann_evaluation_to_json;
use crate::context::view_for_scope;

pub fn search_vector_eval(
    path: &str,
    scope: &str,
    vector: &str,
    json: bool,
    policy: Option<AnnSearchPolicy>,
) -> Result<String, String> {
    let vector = parse_vector_literal(vector)?;
    let db = Database::open(path).map_err(|error| error.to_string())?;
    let report = db
        .evaluate_vector_ann_with_policy(
            &vector,
            &view_for_scope(scope),
            SearchLimit(20),
            policy.unwrap_or_default(),
        )
        .map_err(|error| error.to_string())?;
    if json {
        return Ok(match report {
            Some(report) => ann_evaluation_to_json(
                true,
                None,
                Some(to_ann_search_report(&report)),
                report.exact_top_k.clone(),
                report.ann_top_k.clone(),
                report.overlap_count,
                report.recall_q16,
            ),
            None => ann_evaluation_to_json(
                false,
                Some("requires_persisted_checkpoint_without_wal_tail".to_owned()),
                None,
                Vec::new(),
                Vec::new(),
                0,
                0,
            ),
        });
    }
    Ok(match report {
        Some(report) => format_ann_evaluation(&report),
        None => {
            "ann_evaluation available=false reason=requires_persisted_checkpoint_without_wal_tail"
                .to_owned()
        }
    })
}

fn format_ann_evaluation(report: &AnnEvaluationReport) -> String {
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
    format!(
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
    )
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

fn parse_option_bool(name: &str, value: Option<String>) -> Result<Option<bool>, String> {
    value
        .map(|value| match value.to_ascii_lowercase().as_str() {
            "true" | "1" | "yes" | "on" => Ok(true),
            "false" | "0" | "no" | "off" => Ok(false),
            _ => Err(format!("{name} must be true/false")),
        })
        .transpose()
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
mod tests {
    use super::parse_ann_policy;

    #[test]
    fn parse_ann_policy_default_values() {
        let policy = parse_ann_policy(None, None, None, None, false).unwrap();
        assert!(policy.fallback);
        assert_eq!(policy.min_recall_q16, Some(49_151));
        assert!(!policy.require_slo);
    }

    #[test]
    fn parse_ann_policy_custom_values() {
        let policy = parse_ann_policy(
            Some("false".to_owned()),
            Some(123),
            Some("75%".to_owned()),
            Some(100),
            true,
        )
        .unwrap();
        assert!(!policy.fallback);
        assert_eq!(policy.fallback_scan_cap, Some(123));
        assert_eq!(policy.min_recall_q16, Some(49_151));
        assert_eq!(policy.max_visited_candidates, Some(100));
        assert!(policy.require_slo);
    }

    #[test]
    fn parse_ann_policy_rejects_invalid_bool() {
        let err = parse_ann_policy(Some("maybe".to_owned()), None, None, None, false).unwrap_err();
        assert!(err.contains("fallback must be true/false"));
    }
}
