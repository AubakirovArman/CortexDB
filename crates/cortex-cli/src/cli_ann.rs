use cortex_engine::{parse_vector_literal, AnnEvaluationReport, Database, SearchLimit};

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
        return Ok(ann_evaluation_json(report));
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

fn ann_evaluation_json(report: Option<AnnEvaluationReport>) -> String {
    match report {
        Some(report) => serde_json::json!({
            "available": true,
            "reason": null,
            "ann_report": {
                "path": report.search.path.as_str(),
                "fallback_reason": report.search.fallback_reason.map(|reason| reason.as_str()),
                "requested_limit": report.search.requested_limit,
                "allowed_candidates": report.search.allowed_candidates,
                "graph_nodes": report.search.graph_nodes,
                "returned_candidates": report.search.returned_candidates,
                "recall_q16": report.search.recall_q16,
                "min_recall_q16": report.search.min_recall_q16,
            },
            "exact_top_k": report.exact_top_k,
            "ann_top_k": report.ann_top_k,
            "overlap_count": report.overlap_count,
            "recall_q16": report.recall_q16,
        })
        .to_string(),
        None => serde_json::json!({
            "available": false,
            "reason": "requires_persisted_checkpoint_without_wal_tail",
            "ann_report": null,
            "exact_top_k": [],
            "ann_top_k": [],
            "overlap_count": 0,
            "recall_q16": 0,
        })
        .to_string(),
    }
}
