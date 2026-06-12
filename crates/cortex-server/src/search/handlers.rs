use cortex_aql::AgentView;
use cortex_engine::{
    parse_vector_literal, route_search_query, Database, SearchLimit, SearchMode, SearchQuery,
    SearchRouteInput, SearchRouteStrategy,
};

use crate::authz;
use crate::responses::{AnnEvaluationResponse, RouterError};
use crate::router::{query_param_decoded, query_param_opt_decoded};

use super::params::{
    parse_ann_policy, parse_limit, parse_rerank_mode, resolve_no_fallback_rollout_policy,
};
use super::response::{encode_response, report_response, rollout_decision};

pub fn handle_search_shared(
    db: &Database,
    query: &str,
    body: &[u8],
    authenticated_view: Option<&AgentView>,
) -> Result<String, RouterError> {
    let scope = query_param_decoded(query, "scope").map_err(RouterError::BadRequest)?;
    let view = authz::read_view_for_scope(&scope, authenticated_view)?;
    let limit = query_param_decoded(query, "limit")
        .ok()
        .map(|s| parse_limit(&s))
        .transpose()
        .map_err(RouterError::BadRequest)?
        .unwrap_or(20);
    let mode = query_param_decoded(query, "mode").unwrap_or_else(|_| "keyword".to_owned());
    let algorithm = query_param_decoded(query, "algorithm").unwrap_or_else(|_| "ann".to_owned());
    let rerank = parse_rerank_mode(query).map_err(RouterError::BadRequest)?;
    let ann_policy = parse_ann_policy(query).map_err(RouterError::BadRequest)?;
    let rollout_policy =
        resolve_no_fallback_rollout_policy(db, query).map_err(RouterError::BadRequest)?;
    let q = query_param_opt_decoded(query, "q")
        .unwrap_or_else(|| String::from_utf8_lossy(body).into_owned());
    let vector_literal = query_param_opt_decoded(query, "vector");
    let decision = route_search_query(SearchRouteInput {
        requested_mode: &mode,
        algorithm: &algorithm,
        text_available: !q.trim().is_empty(),
        vector_available: vector_literal
            .as_deref()
            .map(|value| !value.trim().is_empty())
            .unwrap_or(false),
    })
    .map_err(RouterError::BadRequest)?;

    let candidate_limit = rerank.candidate_limit(limit);
    let mut rerank_vector: Option<Vec<i16>> = None;
    let (mut results, ann_report, no_fallback_decision) = match decision.selected_strategy {
        SearchRouteStrategy::Keyword => (
            db.search_keyword(&q, &view, SearchLimit(candidate_limit))?,
            None,
            None,
        ),
        SearchRouteStrategy::VectorExact => {
            let vector =
                vector_literal.unwrap_or_else(|| String::from_utf8_lossy(body).into_owned());
            let vector = parse_vector_literal(&vector).map_err(RouterError::BadRequest)?;
            (
                db.search_vector_exact(&vector, &view, SearchLimit(candidate_limit))?,
                None,
                None,
            )
        }
        SearchRouteStrategy::VectorAnn => {
            let vector =
                vector_literal.unwrap_or_else(|| String::from_utf8_lossy(body).into_owned());
            let vector = parse_vector_literal(&vector).map_err(RouterError::BadRequest)?;
            rerank_vector = Some(vector.clone());
            let outcome = db.search_vector_with_report_with_policy(
                &vector,
                &view,
                SearchLimit(candidate_limit),
                ann_policy,
            )?;
            let no_fallback_decision = outcome
                .ann_report
                .as_ref()
                .and_then(|report| rollout_decision(rollout_policy, ann_policy, report));
            (outcome.results, outcome.ann_report, no_fallback_decision)
        }
        SearchRouteStrategy::Hybrid => {
            let vector = vector_literal.as_deref().ok_or_else(|| {
                RouterError::BadRequest("mode=hybrid requires vector=<i16,...>".to_owned())
            })?;
            let vector = parse_vector_literal(vector).map_err(RouterError::BadRequest)?;
            rerank_vector = Some(vector.clone());
            (
                db.search_cells(
                    SearchQuery {
                        text: &q,
                        vector: Some(&vector),
                        limit: candidate_limit,
                        mode: SearchMode::Hybrid,
                    },
                    &view,
                )?,
                None,
                None,
            )
        }
    };
    rerank.apply(&mut results, &q, rerank_vector.as_deref(), limit);

    encode_response(
        decision.search_mode(),
        Some(decision),
        rerank.response_label(),
        results,
        ann_report,
        no_fallback_decision,
    )
}

pub fn handle_ann_evaluate_shared(
    db: &Database,
    query: &str,
    body: &[u8],
    authenticated_view: Option<&AgentView>,
) -> Result<String, RouterError> {
    let scope = query_param_decoded(query, "scope").map_err(RouterError::BadRequest)?;
    let view = authz::read_view_for_scope(&scope, authenticated_view)?;
    let limit = query_param_decoded(query, "limit")
        .ok()
        .map(|s| parse_limit(&s))
        .transpose()
        .map_err(RouterError::BadRequest)?
        .unwrap_or(20);
    let vector = query_param_decoded(query, "vector")
        .unwrap_or_else(|_| String::from_utf8_lossy(body).into_owned());
    let vector = parse_vector_literal(&vector).map_err(RouterError::BadRequest)?;
    let ann_policy = parse_ann_policy(query).map_err(RouterError::BadRequest)?;
    let rollout_policy =
        resolve_no_fallback_rollout_policy(db, query).map_err(RouterError::BadRequest)?;
    let response =
        match db.evaluate_vector_ann_with_policy(&vector, &view, SearchLimit(limit), ann_policy)? {
            Some(report) => {
                let no_fallback_decision =
                    rollout_decision(rollout_policy, ann_policy, &report.search);
                AnnEvaluationResponse {
                    available: true,
                    reason: None,
                    ann_report: Some(report_response(report.search)),
                    no_fallback_decision,
                    exact_top_k: report.exact_top_k,
                    ann_top_k: report.ann_top_k,
                    overlap_count: report.overlap_count,
                    recall_q16: report.recall_q16,
                }
            }
            None => AnnEvaluationResponse {
                available: false,
                reason: Some("requires_persisted_checkpoint_without_wal_tail".to_owned()),
                ann_report: None,
                no_fallback_decision: None,
                exact_top_k: Vec::new(),
                ann_top_k: Vec::new(),
                overlap_count: 0,
                recall_q16: 0,
            },
        };
    Ok(serde_json::to_string(&response)?)
}
