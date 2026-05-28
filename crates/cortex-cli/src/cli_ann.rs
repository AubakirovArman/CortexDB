use cortex_engine::{parse_vector_literal, AnnEvaluationReport, Database, SearchLimit};

use crate::cli_json::ann_evaluation_to_json;
use crate::context::view_for_scope;

pub fn search_vector_eval(
    path: &str,
    scope: &str,
    vector: &str,
    json: bool,
) -> Result<String, String> {
    let vector = parse_vector_literal(vector)?;
    let db = Database::open(path).map_err(|error| error.to_string())?;
    let report = db
        .evaluate_vector_ann(&vector, &view_for_scope(scope), SearchLimit(20))
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
