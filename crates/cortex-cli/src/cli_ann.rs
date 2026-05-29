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
    format!(
        "ann_evaluation available=true path={} fallback_reason={} recall_q16={} min_recall_q16={} overlap_count={} exact_top_k={:?} ann_top_k={:?}",
        report.search.path.as_str(),
        report
            .search
            .fallback_reason
            .map(|reason| reason.as_str())
            .unwrap_or("null"),
        report.recall_q16,
        report
            .search
            .min_recall_q16
            .map(|value| value.to_string())
            .unwrap_or_else(|| "null".to_owned()),
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
        recall_q16: report.search.recall_q16,
        min_recall_q16: report.search.min_recall_q16,
    }
}

pub(crate) fn parse_ann_policy(
    fallback: Option<String>,
    fallback_scan_cap: Option<usize>,
    min_recall: Option<String>,
) -> Result<AnnSearchPolicy, String> {
    let default_policy = AnnSearchPolicy::default();
    let fallback = parse_option_bool(fallback)?.unwrap_or(default_policy.fallback);
    let min_recall_q16 = match min_recall {
        Some(value) => Some(parse_min_recall_q16(&value)?),
        None => default_policy.min_recall_q16,
    };
    Ok(AnnSearchPolicy {
        min_recall_q16,
        fallback,
        fallback_scan_cap,
    })
}

fn parse_option_bool(value: Option<String>) -> Result<Option<bool>, String> {
    value
        .map(|value| match value.to_ascii_lowercase().as_str() {
            "true" | "1" | "yes" | "on" => Ok(true),
            "false" | "0" | "no" | "off" => Ok(false),
            _ => Err("fallback must be true/false".to_owned()),
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
        let policy = parse_ann_policy(None, None, None).unwrap();
        assert!(policy.fallback);
        assert_eq!(policy.min_recall_q16, Some(49_151));
    }

    #[test]
    fn parse_ann_policy_custom_values() {
        let policy =
            parse_ann_policy(Some("false".to_owned()), Some(123), Some("75%".to_owned())).unwrap();
        assert!(!policy.fallback);
        assert_eq!(policy.fallback_scan_cap, Some(123));
        assert_eq!(policy.min_recall_q16, Some(49_151));
    }

    #[test]
    fn parse_ann_policy_rejects_invalid_bool() {
        let err = parse_ann_policy(Some("maybe".to_owned()), None, None).unwrap_err();
        assert!(err.contains("fallback must be true/false"));
    }
}
