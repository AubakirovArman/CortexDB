use cortex_engine::{parse_vector_literal, AnnSearchReport, Database, SearchLimit};

use crate::context::view_for_scope;
use crate::responses::{
    AnnEvaluationResponse, AnnSearchReportResponse, SearchResponse, SearchResultResponse,
};

pub fn handle_search_shared(
    db: &std::sync::RwLock<Database>,
    query: &str,
    body: &[u8],
) -> Result<String, String> {
    let scope = query_param(query, "scope")?;
    let limit = query_param(query, "limit")
        .ok()
        .map(parse_limit)
        .transpose()?
        .unwrap_or(20);
    let mode = query_param(query, "mode").unwrap_or("keyword");
    let algorithm = query_param(query, "algorithm").unwrap_or("ann");
    let db = db.read().map_err(|e| e.to_string())?;
    let (search_mode, results, ann_report) = match mode {
        "keyword" => (
            "keyword",
            {
                let text = query_param(query, "q")
                    .map(str::to_owned)
                    .unwrap_or_else(|_| String::from_utf8_lossy(body).into_owned());
                db.search_keyword(&text, &view_for_scope(scope), SearchLimit(limit))
            },
            None,
        ),
        "vector" => {
            let vector = query_param(query, "vector")
                .map(str::to_owned)
                .unwrap_or_else(|_| String::from_utf8_lossy(body).into_owned());
            let vector = parse_vector_literal(&vector)?;
            match algorithm {
                "exact" => (
                    "vector_exact",
                    db.search_vector_exact(&vector, &view_for_scope(scope), SearchLimit(limit)),
                    None,
                ),
                "ann" => {
                    let outcome = db
                        .search_vector_with_report(
                            &vector,
                            &view_for_scope(scope),
                            SearchLimit(limit),
                        )
                        .map_err(|error| error.to_string())?;
                    return encode_response("vector_ann", outcome.results, outcome.ann_report);
                }
                _ => return Err("algorithm must be exact or ann".to_owned()),
            }
        }
        _ => return Err("mode must be keyword or vector".to_owned()),
    };
    let results = results.map_err(|error| error.to_string())?;
    encode_response(search_mode, results, ann_report)
}

pub fn handle_ann_evaluate_shared(
    db: &std::sync::RwLock<Database>,
    query: &str,
    body: &[u8],
) -> Result<String, String> {
    let scope = query_param(query, "scope")?;
    let limit = query_param(query, "limit")
        .ok()
        .map(parse_limit)
        .transpose()?
        .unwrap_or(20);
    let vector = query_param(query, "vector")
        .map(str::to_owned)
        .unwrap_or_else(|_| String::from_utf8_lossy(body).into_owned());
    let vector = parse_vector_literal(&vector)?;
    let db = db.read().map_err(|e| e.to_string())?;
    let response = match db
        .evaluate_vector_ann(&vector, &view_for_scope(scope), SearchLimit(limit))
        .map_err(|error| error.to_string())?
    {
        Some(report) => AnnEvaluationResponse {
            available: true,
            reason: None,
            ann_report: Some(report_response(report.search)),
            exact_top_k: report.exact_top_k,
            ann_top_k: report.ann_top_k,
            overlap_count: report.overlap_count,
            recall_q16: report.recall_q16,
        },
        None => AnnEvaluationResponse {
            available: false,
            reason: Some("requires_persisted_checkpoint_without_wal_tail".to_owned()),
            ann_report: None,
            exact_top_k: Vec::new(),
            ann_top_k: Vec::new(),
            overlap_count: 0,
            recall_q16: 0,
        },
    };
    serde_json::to_string(&response).map_err(|e| e.to_string())
}

fn encode_response(
    search_mode: &str,
    results: Vec<cortex_engine::DatabaseSearchResult>,
    ann_report: Option<AnnSearchReport>,
) -> Result<String, String> {
    let response = SearchResponse {
        search_mode: search_mode.to_owned(),
        ann_report: ann_report.map(report_response),
        results: results
            .iter()
            .map(|result| SearchResultResponse {
                cell_id: result.cell_id.0,
                score: result.score,
                lexical_score: result.lexical_score,
                vector_score: result.vector_score,
                payload: String::from_utf8_lossy(&result.payload).into_owned(),
            })
            .collect(),
    };
    serde_json::to_string(&response).map_err(|e| e.to_string())
}

fn report_response(report: AnnSearchReport) -> AnnSearchReportResponse {
    AnnSearchReportResponse {
        path: report.path.as_str().to_owned(),
        fallback_reason: report
            .fallback_reason
            .map(|reason| reason.as_str().to_owned()),
        requested_limit: report.requested_limit,
        allowed_candidates: report.allowed_candidates,
        graph_nodes: report.graph_nodes,
        returned_candidates: report.returned_candidates,
        recall_q16: report.recall_q16,
        min_recall_q16: report.min_recall_q16,
    }
}

fn parse_limit(value: &str) -> Result<usize, String> {
    value
        .parse::<usize>()
        .map(|limit| limit.max(1))
        .map_err(|_| "limit must be usize".to_owned())
}

fn query_param<'a>(query: &'a str, key: &str) -> Result<&'a str, String> {
    let prefix = format!("{key}=");
    query
        .split('&')
        .find_map(|pair| pair.strip_prefix(&prefix))
        .ok_or_else(|| format!("missing {key}"))
}
