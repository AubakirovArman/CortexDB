use std::collections::{BTreeMap, BTreeSet};

use cortex_aql::AgentView;
use cortex_engine::{
    parse_vector_literal, route_search_query, tokenize, CellMetadata, Database,
    DatabaseSearchResult, SearchLimit, SearchMode, SearchQuery, SearchRouteInput,
    SearchRouteStrategy,
};

use crate::authz;
use crate::responses::{
    RouterError, SearchExplainItemResponse, SearchExplainResponse,
    SearchExplainTermContributionResponse,
};
use crate::router::{query_param_decoded, query_param_opt_decoded};

use super::params::{parse_limit, parse_rerank_mode};
use super::response::routing_response;

pub fn handle_search_explain_shared(
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
    let q = query_param_decoded(query, "q")
        .unwrap_or_else(|_| String::from_utf8_lossy(body).into_owned());
    let vector_literal = query_param_opt_decoded(query, "vector");
    let route = route_search_query(SearchRouteInput {
        requested_mode: &mode,
        algorithm: &algorithm,
        text_available: !q.trim().is_empty(),
        vector_available: vector_literal
            .as_deref()
            .map(|value| !value.trim().is_empty())
            .unwrap_or(false),
    })
    .map_err(RouterError::BadRequest)?;

    let query_terms = tokenize(&q);

    let candidate_limit = rerank.candidate_limit(limit);
    let mut results = match route.selected_strategy {
        SearchRouteStrategy::Keyword => db.search_keyword(&q, &view, SearchLimit(candidate_limit)),
        SearchRouteStrategy::VectorExact => {
            let vector = vector_literal.clone().unwrap_or_else(|| q.clone());
            let v = parse_vector_literal(&vector).map_err(RouterError::BadRequest)?;
            db.search_vector_exact(&v, &view, SearchLimit(candidate_limit))
        }
        SearchRouteStrategy::VectorAnn => {
            let vector = vector_literal.clone().unwrap_or_else(|| q.clone());
            let v = parse_vector_literal(&vector).map_err(RouterError::BadRequest)?;
            db.search_vector(&v, &view, SearchLimit(candidate_limit))
        }
        SearchRouteStrategy::Hybrid => {
            let vector = vector_literal.ok_or_else(|| {
                RouterError::BadRequest("mode=hybrid requires vector=<i16,...>".to_owned())
            })?;
            let v = parse_vector_literal(&vector).map_err(RouterError::BadRequest)?;
            db.search_cells(
                SearchQuery {
                    text: &q,
                    vector: Some(&v),
                    limit: candidate_limit,
                    mode: SearchMode::Hybrid,
                },
                &view,
            )
        }
    }?;
    rerank.apply(&mut results, &q, None, limit);

    let explain_results = results
        .iter()
        .enumerate()
        .map(|(index, result)| explain_item(index + 1, result, &query_terms, route.search_mode()))
        .collect();
    let response = SearchExplainResponse {
        query_terms,
        search_mode: route.search_mode().to_owned(),
        routing: Some(routing_response(route)),
        results: explain_results,
    };
    Ok(serde_json::to_string(&response)?)
}

fn explain_item(
    rank: usize,
    result: &DatabaseSearchResult,
    query_terms: &[String],
    mode: &str,
) -> SearchExplainItemResponse {
    let metadata = &result.metadata;
    let payload_terms = metadata.weighted_lexical_terms();
    let matched_terms = query_terms
        .iter()
        .filter(|term| payload_terms.contains_key(term.as_str()))
        .cloned()
        .collect::<Vec<_>>();
    let term_contributions = term_contributions(&matched_terms, &payload_terms, result);
    let matched_fields = matched_fields(metadata, query_terms, result);
    SearchExplainItemResponse {
        cell_id: result.cell_id.0,
        rank,
        score: result.score,
        lexical_score: result.lexical_score,
        vector_score: result.vector_score,
        lexical_contribution_q16: contribution_q16(result.lexical_score, result),
        vector_contribution_q16: contribution_q16(result.vector_score, result),
        fusion_rank_score: fusion_rank_score(result),
        matched_terms,
        matched_fields,
        term_contributions,
        contribution_summary: contribution_summary(mode, result),
        payload_preview: truncate_preview(&result.payload, 200),
    }
}

fn matched_fields(
    metadata: &CellMetadata,
    query_terms: &[String],
    result: &DatabaseSearchResult,
) -> Vec<String> {
    let mut fields = Vec::new();
    if field_matches(metadata.title.as_deref(), query_terms) {
        fields.push("title".to_owned());
    }
    if field_matches(Some(&metadata.body_text), query_terms) {
        fields.push("body_text".to_owned());
    }
    if result.vector_score > 0 {
        fields.push("vector".to_owned());
    }
    fields
}

fn field_matches(text: Option<&str>, query_terms: &[String]) -> bool {
    let Some(text) = text else {
        return false;
    };
    let terms = tokenize(text).into_iter().collect::<BTreeSet<_>>();
    query_terms.iter().any(|term| terms.contains(term))
}

fn term_contributions(
    matched_terms: &[String],
    payload_terms: &BTreeMap<String, u32>,
    result: &DatabaseSearchResult,
) -> Vec<SearchExplainTermContributionResponse> {
    let total = matched_terms
        .iter()
        .filter_map(|term| payload_terms.get(term))
        .copied()
        .sum::<u32>()
        .max(1);
    matched_terms
        .iter()
        .map(|term| {
            let frequency = *payload_terms.get(term).unwrap_or(&0);
            SearchExplainTermContributionResponse {
                term: term.clone(),
                term_frequency: frequency,
                score: result.lexical_score * u64::from(frequency) / u64::from(total),
            }
        })
        .collect()
}

fn contribution_q16(component: u64, result: &DatabaseSearchResult) -> u16 {
    let total = result.lexical_score.saturating_add(result.vector_score);
    component
        .saturating_mul(65_535)
        .checked_div(total)
        .unwrap_or(0)
        .min(65_535) as u16
}

fn fusion_rank_score(result: &DatabaseSearchResult) -> u64 {
    if result.lexical_score > 0 && result.vector_score > 0 {
        result.score
    } else {
        0
    }
}

fn contribution_summary(mode: &str, result: &DatabaseSearchResult) -> String {
    match mode {
        "keyword" => format!("keyword lexical_score={}", result.lexical_score),
        "vector" => format!("vector similarity_score={}", result.vector_score),
        "hybrid" => format!(
            "hybrid rrf_score={} lexical_score={} vector_score={}",
            result.score, result.lexical_score, result.vector_score
        ),
        _ => format!("score={}", result.score),
    }
}

fn truncate_preview(payload: &[u8], max_len: usize) -> String {
    let s = String::from_utf8_lossy(payload);
    if s.len() <= max_len {
        s.into_owned()
    } else {
        format!("{}...", &s[..max_len])
    }
}
