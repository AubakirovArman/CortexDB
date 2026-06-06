use cortex_aql::AgentView;
use cortex_engine::{
    evaluate_hnsw_no_fallback_rollout, parse_vector_literal, route_search_query, tokenize,
    AnnSearchPolicy, AnnSearchReport, CellMetadata, Database, DatabaseSearchResult,
    HnswNoFallbackDecision, HnswNoFallbackRolloutPolicy, SearchLimit, SearchMode, SearchQuery,
    SearchRouteDecision, SearchRouteInput, SearchRouteStrategy,
};
use std::collections::{BTreeMap, BTreeSet};

use crate::authz;
use crate::responses::{
    AnnEvaluationResponse, AnnNoFallbackDecisionResponse, AnnSearchReportResponse, RouterError,
    SearchExplainItemResponse, SearchExplainResponse, SearchExplainTermContributionResponse,
    SearchResponse, SearchResultResponse, SearchRoutingDecisionResponse,
};
use crate::router::{query_param_decoded, query_param_opt, query_param_opt_decoded};

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

    let results = match route.selected_strategy {
        SearchRouteStrategy::Keyword => db.search_keyword(&q, &view, SearchLimit(limit)),
        SearchRouteStrategy::VectorExact => {
            let vector = vector_literal.clone().unwrap_or_else(|| q.clone());
            let v = parse_vector_literal(&vector).map_err(RouterError::BadRequest)?;
            db.search_vector_exact(&v, &view, SearchLimit(limit))
        }
        SearchRouteStrategy::VectorAnn => {
            let vector = vector_literal.clone().unwrap_or_else(|| q.clone());
            let v = parse_vector_literal(&vector).map_err(RouterError::BadRequest)?;
            db.search_vector(&v, &view, SearchLimit(limit))
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
                    limit,
                    mode: SearchMode::Hybrid,
                },
                &view,
            )
        }
    }?;

    let explain_results = results
        .iter()
        .enumerate()
        .map(|(index, result)| explain_item(index + 1, result, &query_terms, route.search_mode()))
        .collect();
    let response = SearchExplainResponse {
        query_terms,
        search_mode: route.search_mode().to_owned(),
        routing: routing_response(route),
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
    let metadata = CellMetadata::from_payload(&result.payload);
    let payload_terms = metadata.weighted_lexical_terms();
    let matched_terms = query_terms
        .iter()
        .filter(|term| payload_terms.contains_key(term.as_str()))
        .cloned()
        .collect::<Vec<_>>();
    let term_contributions = term_contributions(&matched_terms, &payload_terms, result);
    let matched_fields = matched_fields(&metadata, query_terms, result);
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

    let (results, ann_report, no_fallback_decision) = match decision.selected_strategy {
        SearchRouteStrategy::Keyword => (
            db.search_keyword(&q, &view, SearchLimit(limit))?,
            None,
            None,
        ),
        SearchRouteStrategy::VectorExact => {
            let vector =
                vector_literal.unwrap_or_else(|| String::from_utf8_lossy(body).into_owned());
            let vector = parse_vector_literal(&vector).map_err(RouterError::BadRequest)?;
            (
                db.search_vector_exact(&vector, &view, SearchLimit(limit))?,
                None,
                None,
            )
        }
        SearchRouteStrategy::VectorAnn => {
            let vector =
                vector_literal.unwrap_or_else(|| String::from_utf8_lossy(body).into_owned());
            let vector = parse_vector_literal(&vector).map_err(RouterError::BadRequest)?;
            let outcome = db.search_vector_with_report_with_policy(
                &vector,
                &view,
                SearchLimit(limit),
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
            (
                db.search_cells(
                    SearchQuery {
                        text: &q,
                        vector: Some(&vector),
                        limit,
                        mode: SearchMode::Hybrid,
                    },
                    &view,
                )?,
                None,
                None,
            )
        }
    };

    encode_response(
        decision.search_mode(),
        Some(decision),
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

fn encode_response(
    search_mode: &str,
    routing: Option<SearchRouteDecision>,
    results: Vec<cortex_engine::DatabaseSearchResult>,
    ann_report: Option<AnnSearchReport>,
    no_fallback_decision: Option<AnnNoFallbackDecisionResponse>,
) -> Result<String, RouterError> {
    let response = SearchResponse {
        search_mode: search_mode.to_owned(),
        routing: routing.map(routing_response),
        ann_report: ann_report.map(report_response),
        no_fallback_decision,
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
    Ok(serde_json::to_string(&response)?)
}

fn routing_response(decision: SearchRouteDecision) -> SearchRoutingDecisionResponse {
    SearchRoutingDecisionResponse {
        requested_mode: decision.requested_mode,
        selected_strategy: decision.selected_strategy.as_str().to_owned(),
        reason: decision.reason.to_owned(),
        text_available: decision.text_available,
        vector_available: decision.vector_available,
    }
}

fn report_response(report: AnnSearchReport) -> AnnSearchReportResponse {
    AnnSearchReportResponse {
        path: report.path.as_str().to_owned(),
        fallback_reason: report
            .fallback_reason
            .map(|reason| reason.as_str().to_owned()),
        fallback_performed: report.fallback_performed,
        requested_limit: report.requested_limit,
        allowed_candidates: report.allowed_candidates,
        graph_nodes: report.graph_nodes,
        returned_candidates: report.returned_candidates,
        visited_candidates: report.visited_candidates,
        max_visited_candidates: report.max_visited_candidates,
        recall_q16: report.recall_q16,
        min_recall_q16: report.min_recall_q16,
        hnsw_max_neighbors: report.hnsw_max_neighbors,
        hnsw_ef_search: report.hnsw_ef_search,
        hnsw_ef_construction: report.hnsw_ef_construction,
        hnsw_layer_count: report.hnsw_layer_count,
        upper_graph_edges: report.upper_graph_edges,
        require_slo: report.require_slo,
        production_safe: report.production_safe,
        slo_violations: report
            .slo_violations
            .iter()
            .map(|violation| violation.as_str().to_owned())
            .collect(),
    }
}

fn rollout_decision(
    rollout_policy: Option<HnswNoFallbackRolloutPolicy>,
    ann_policy: AnnSearchPolicy,
    report: &AnnSearchReport,
) -> Option<AnnNoFallbackDecisionResponse> {
    rollout_policy.map(|policy| {
        no_fallback_decision_response(evaluate_hnsw_no_fallback_rollout(
            policy, ann_policy, report,
        ))
    })
}

fn no_fallback_decision_response(
    decision: HnswNoFallbackDecision,
) -> AnnNoFallbackDecisionResponse {
    AnnNoFallbackDecisionResponse {
        allowed: decision.allowed,
        reasons: decision
            .reasons
            .iter()
            .map(|reason| reason.as_str().to_owned())
            .collect(),
    }
}

fn parse_ann_policy(query: &str) -> Result<AnnSearchPolicy, String> {
    let default_policy = AnnSearchPolicy::default();
    let fallback = parse_optional_query_param(query, "fallback")?
        .map(|value| parse_bool("fallback", &value))
        .transpose()?
        .unwrap_or(default_policy.fallback);
    let fallback_scan_cap = parse_optional_query_param(query, "fallback_scan_cap")?
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|_| "fallback_scan_cap must be usize".to_owned())
        })
        .transpose()?;
    let min_recall_q16 = parse_optional_query_param(query, "min_recall")?
        .map(|value| parse_min_recall_q16(&value))
        .transpose()?
        .or(default_policy.min_recall_q16);

    let max_visited_candidates = parse_optional_query_param(query, "max_visited_candidates")?
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|_| "max_visited_candidates must be usize".to_owned())
        })
        .transpose()?;

    let require_slo = parse_optional_query_param(query, "require_slo")?
        .map(|value| parse_bool("require_slo", &value))
        .transpose()?
        .unwrap_or(default_policy.require_slo);

    Ok(AnnSearchPolicy {
        min_recall_q16,
        fallback,
        fallback_scan_cap,
        max_visited_candidates,
        require_slo,
    })
}

fn resolve_no_fallback_rollout_policy(
    db: &Database,
    query: &str,
) -> Result<Option<HnswNoFallbackRolloutPolicy>, String> {
    let use_profile = parse_optional_query_param(query, "no_fallback_profile")?
        .map(|value| parse_profile_selector(&value))
        .transpose()?
        .unwrap_or(false);
    let rollout = parse_optional_query_param(query, "no_fallback_rollout")?
        .map(|value| parse_bool("no_fallback_rollout", &value))
        .transpose()?;
    let min_recall = parse_optional_query_param(query, "no_fallback_min_recall")?
        .map(|value| parse_min_recall_q16(&value))
        .transpose()?;
    if use_profile && (rollout.is_some() || min_recall.is_some()) {
        return Err("no_fallback_profile cannot be combined with no_fallback_rollout or no_fallback_min_recall".to_owned());
    }
    if use_profile {
        return db
            .hnsw_no_fallback_rollout_policy()
            .map(Some)
            .ok_or_else(|| "no persisted HNSW no-fallback profile is configured".to_owned());
    }
    if rollout.is_none() && min_recall.is_none() {
        return Ok(None);
    }
    if rollout != Some(true) && min_recall.is_some() {
        return Err("no_fallback_min_recall requires no_fallback_rollout=true".to_owned());
    }
    let default_policy = HnswNoFallbackRolloutPolicy::default();
    let policy = HnswNoFallbackRolloutPolicy {
        rollout_enabled: rollout.unwrap_or(false),
        min_recall_q16: min_recall.unwrap_or(default_policy.min_recall_q16),
        require_upper_layers: default_policy.require_upper_layers,
    };
    Ok(Some(policy))
}

fn parse_profile_selector(value: &str) -> Result<bool, String> {
    match value.to_ascii_lowercase().as_str() {
        "active" | "true" | "1" | "yes" | "on" => Ok(true),
        "false" | "0" | "no" | "off" => Ok(false),
        _ => Err("no_fallback_profile must be active or true/false".to_owned()),
    }
}

fn parse_optional_query_param(query: &str, key: &str) -> Result<Option<String>, String> {
    if query_param_opt(query, key).is_some() {
        query_param_decoded(query, key).map(Some)
    } else {
        Ok(None)
    }
}

fn parse_bool(name: &str, value: &str) -> Result<bool, String> {
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
