use cortex_engine::{
    parse_vector_literal, AnnSearchPolicy, AnnSearchReport, Database, SearchLimit,
};

use crate::context::view_for_scope;
use crate::responses::{
    AnnEvaluationResponse, AnnSearchReportResponse, SearchExplainItemResponse,
    SearchExplainResponse, SearchResponse, SearchResultResponse,
};
use crate::router::{query_param_decoded, query_param_opt};

pub fn handle_search_explain_shared(
    db: &Database,
    query: &str,
    body: &[u8],
) -> Result<String, String> {
    let scope = query_param_decoded(query, "scope")?;
    let limit = query_param_decoded(query, "limit")
        .ok()
        .map(|s| parse_limit(&s))
        .transpose()?
        .unwrap_or(20);
    let mode = query_param_decoded(query, "mode").unwrap_or_else(|_| "keyword".to_owned());
    let q = query_param_decoded(query, "q")
        .unwrap_or_else(|_| String::from_utf8_lossy(body).into_owned());

    let diagnostics = db.search_diagnostics(&q).map_err(|e| e.to_string())?;
    let query_terms = extract_terms_from_diagnostics(&diagnostics);

    let results = match mode.as_str() {
        "keyword" => db.search_keyword(&q, &view_for_scope(&scope), SearchLimit(limit)),
        "vector" => {
            let v = parse_vector_literal(&q)?;
            db.search_vector(&v, &view_for_scope(&scope), SearchLimit(limit))
        }
        _ => return Err("mode must be keyword or vector".to_owned()),
    }
    .map_err(|e| e.to_string())?;

    let response = SearchExplainResponse {
        query_terms,
        search_mode: mode,
        results: results
            .iter()
            .map(|r| SearchExplainItemResponse {
                cell_id: r.cell_id.0,
                score: r.score,
                lexical_score: r.lexical_score,
                vector_score: r.vector_score,
                payload_preview: truncate_preview(&r.payload, 200),
            })
            .collect(),
    };
    serde_json::to_string(&response).map_err(|e| e.to_string())
}

fn extract_terms_from_diagnostics(diagnostics: &str) -> Vec<String> {
    diagnostics
        .split("terms=[")
        .nth(1)
        .and_then(|s| s.strip_suffix(']'))
        .map(|terms| terms.split(", ").map(|t| t.to_owned()).collect())
        .unwrap_or_default()
}

fn truncate_preview(payload: &[u8], max_len: usize) -> String {
    let s = String::from_utf8_lossy(payload);
    if s.len() <= max_len {
        s.into_owned()
    } else {
        format!("{}...", &s[..max_len])
    }
}

pub fn handle_search_shared(db: &Database, query: &str, body: &[u8]) -> Result<String, String> {
    let scope = query_param_decoded(query, "scope")?;
    let limit = query_param_decoded(query, "limit")
        .ok()
        .map(|s| parse_limit(&s))
        .transpose()?
        .unwrap_or(20);
    let mode = query_param_decoded(query, "mode").unwrap_or_else(|_| "keyword".to_owned());
    let algorithm = query_param_decoded(query, "algorithm").unwrap_or_else(|_| "ann".to_owned());
    let ann_policy = parse_ann_policy(query)?;
    let view = view_for_scope(&scope);

    let (search_mode, results, ann_report) = match mode.as_str() {
        "keyword" => (
            "keyword",
            db.search_keyword(
                &query_param_decoded(query, "q")
                    .unwrap_or_else(|_| String::from_utf8_lossy(body).into_owned()),
                &view,
                SearchLimit(limit),
            )
            .map_err(|error| error.to_string())?,
            None,
        ),
        "vector" => {
            let vector = query_param_decoded(query, "vector")
                .unwrap_or_else(|_| String::from_utf8_lossy(body).into_owned());
            let vector = parse_vector_literal(&vector)?;
            match algorithm.as_str() {
                "exact" => (
                    "vector_exact",
                    db.search_vector_exact(&vector, &view, SearchLimit(limit))
                        .map_err(|error| error.to_string())?,
                    None,
                ),
                "ann" => {
                    let outcome = db
                        .search_vector_with_report_with_policy(
                            &vector,
                            &view,
                            SearchLimit(limit),
                            ann_policy,
                        )
                        .map_err(|error| error.to_string())?;
                    return encode_response("vector_ann", outcome.results, outcome.ann_report);
                }
                _ => return Err("algorithm must be exact or ann".to_owned()),
            }
        }
        _ => return Err("mode must be keyword or vector".to_owned()),
    };

    encode_response(search_mode, results, ann_report)
}

pub fn handle_ann_evaluate_shared(
    db: &Database,
    query: &str,
    body: &[u8],
) -> Result<String, String> {
    let scope = query_param_decoded(query, "scope")?;
    let limit = query_param_decoded(query, "limit")
        .ok()
        .map(|s| parse_limit(&s))
        .transpose()?
        .unwrap_or(20);
    let vector = query_param_decoded(query, "vector")
        .unwrap_or_else(|_| String::from_utf8_lossy(body).into_owned());
    let vector = parse_vector_literal(&vector)?;
    let response = match db
        .evaluate_vector_ann_with_policy(
            &vector,
            &view_for_scope(&scope),
            SearchLimit(limit),
            parse_ann_policy(query)?,
        )
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

fn parse_ann_policy(query: &str) -> Result<AnnSearchPolicy, String> {
    let default_policy = AnnSearchPolicy::default();
    let fallback = query_param_opt(query, "fallback")
        .map(parse_bool)
        .transpose()?
        .unwrap_or(default_policy.fallback);
    let fallback_scan_cap = query_param_opt(query, "fallback_scan_cap")
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|_| "fallback_scan_cap must be usize".to_owned())
        })
        .transpose()?;
    let min_recall_q16 = query_param_opt(query, "min_recall")
        .map(parse_min_recall_q16)
        .transpose()?
        .or(default_policy.min_recall_q16);

    Ok(AnnSearchPolicy {
        min_recall_q16,
        fallback,
        fallback_scan_cap,
    })
}

fn parse_bool(value: &str) -> Result<bool, String> {
    match value.to_ascii_lowercase().as_str() {
        "true" | "1" | "yes" | "on" => Ok(true),
        "false" | "0" | "no" | "off" => Ok(false),
        _ => Err("fallback must be true/false".to_owned()),
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
    Ok((ratio * f64::from(u16::MAX)) as u16)
}

fn parse_percent_without_unit(value: &str) -> Result<f64, String> {
    value
        .trim()
        .parse::<f64>()
        .map_err(|_| "min_recall must be percentage value".to_owned())
}

fn parse_limit(value: &str) -> Result<usize, String> {
    value
        .parse::<usize>()
        .map(|limit| limit.max(1))
        .map_err(|_| "limit must be usize".to_owned())
}
