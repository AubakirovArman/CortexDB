use cortex_engine::{
    evaluate_hnsw_no_fallback_rollout, parse_vector_literal, route_search_query, AnnSearchPath,
    AnnSearchPolicy, Database, DatabaseSearchResult, HnswNoFallbackRolloutPolicy, SearchLimit,
    SearchMode, SearchQuery, SearchRouteInput, SearchRouteStrategy,
};

use crate::cli_json::no_fallback_profile_to_json;
use crate::context::{format_search_results, view_for_scope};

use super::common::{fmt_engine_error, open_database};

pub fn search(
    path: &str,
    scope: &str,
    query: &str,
    json: bool,
    mode: &str,
    vector: Option<&str>,
    algorithm: &str,
) -> Result<String, String> {
    let db = open_database(path, false)?;
    let route = route_search_query(SearchRouteInput {
        requested_mode: mode,
        algorithm,
        text_available: !query.trim().is_empty(),
        vector_available: vector
            .map(|value| !value.trim().is_empty())
            .unwrap_or(false),
    })?;
    let view = view_for_scope(scope);
    let results = match route.selected_strategy {
        SearchRouteStrategy::Keyword => db.search_keyword(query, &view, SearchLimit(20)),
        SearchRouteStrategy::VectorExact => {
            let vector = parse_vector_literal(vector.unwrap_or(query))?;
            db.search_vector_exact(&vector, &view, SearchLimit(20))
        }
        SearchRouteStrategy::VectorAnn => {
            let vector = parse_vector_literal(vector.unwrap_or(query))?;
            db.search_vector(&vector, &view, SearchLimit(20))
        }
        SearchRouteStrategy::Hybrid => {
            let vector = vector.ok_or_else(|| "mode=hybrid requires --vector".to_owned())?;
            let vector = parse_vector_literal(vector)?;
            db.search_cells(
                SearchQuery {
                    text: query,
                    vector: Some(&vector),
                    limit: 20,
                    mode: SearchMode::Hybrid,
                },
                &view,
            )
        }
    }
    .map_err(fmt_engine_error)?;
    if json {
        Ok(crate::cli_json::search_to_json(
            &results,
            route.search_mode(),
            Some(&route),
        ))
    } else {
        Ok(format!(
            "routing requested_mode={} selected_strategy={} reason={}\n{}",
            route.requested_mode,
            route.selected_strategy.as_str(),
            route.reason,
            format_search_results(&results)
        ))
    }
}

pub fn search_explain(
    path: &str,
    scope: &str,
    query: &str,
    mode: &str,
    vector: Option<&str>,
) -> Result<String, String> {
    let db = open_database(path, false)?;
    let diagnostics = db.search_diagnostics(query).map_err(fmt_engine_error)?;
    let results = match mode {
        "keyword" => db.search_keyword(query, &view_for_scope(scope), SearchLimit(20)),
        "vector" => {
            let v = parse_vector_literal(vector.unwrap_or(query))?;
            db.search_vector(&v, &view_for_scope(scope), SearchLimit(20))
        }
        "hybrid" => {
            let vector = vector.ok_or_else(|| "mode=hybrid requires --vector".to_owned())?;
            let vector = parse_vector_literal(vector)?;
            db.search_cells(
                SearchQuery {
                    text: query,
                    vector: Some(&vector),
                    limit: 20,
                    mode: SearchMode::Hybrid,
                },
                &view_for_scope(scope),
            )
        }
        _ => return Err("mode must be keyword, vector, or hybrid".to_owned()),
    }
    .map_err(fmt_engine_error)?;
    let mut lines = vec![diagnostics];
    for (rank, result) in results.iter().enumerate() {
        lines.push(format_search_explain_line(rank + 1, result));
    }
    Ok(lines.join("\n"))
}

fn format_search_explain_line(rank: usize, result: &DatabaseSearchResult) -> String {
    let total = result.lexical_score.saturating_add(result.vector_score);
    let lexical_q16 = result
        .lexical_score
        .saturating_mul(65_535)
        .checked_div(total)
        .unwrap_or(0);
    let vector_q16 = result
        .vector_score
        .saturating_mul(65_535)
        .checked_div(total)
        .unwrap_or(0);
    let preview = String::from_utf8_lossy(&result.payload)
        .chars()
        .take(80)
        .collect::<String>();
    format!(
        "rank={} cell_id={} score={} lexical={} vector={} lexical_q16={} vector_q16={} fusion={} preview={}",
        rank,
        result.cell_id.0,
        result.score,
        result.lexical_score,
        result.vector_score,
        lexical_q16,
        vector_q16,
        result.lexical_score > 0 && result.vector_score > 0,
        preview
    )
}

pub struct SearchVectorOptions<'a> {
    pub path: &'a str,
    pub scope: &'a str,
    pub vector: &'a str,
    pub exact: bool,
    pub policy: Option<AnnSearchPolicy>,
    pub rollout_policy: Option<HnswNoFallbackRolloutPolicy>,
    pub use_no_fallback_profile: bool,
    pub experimental_hnsw: bool,
}

pub fn search_vector(options: SearchVectorOptions<'_>) -> Result<String, String> {
    let vector = parse_vector_literal(options.vector)?;
    let db = open_database(options.path, options.experimental_hnsw)?;
    let rollout_policy =
        resolve_no_fallback_profile(&db, options.rollout_policy, options.use_no_fallback_profile)?;
    let view = view_for_scope(options.scope);
    if options.exact {
        let results = db
            .search_vector_exact(&vector, &view, SearchLimit(20))
            .map_err(fmt_engine_error)?;
        Ok(format_search_results(&results))
    } else {
        let search_policy = options.policy.unwrap_or_default();
        let outcome = db
            .search_vector_with_report_with_policy(&vector, &view, SearchLimit(20), search_policy)
            .map_err(fmt_engine_error)?;
        let mut lines = Vec::new();
        lines.push(format_search_results(&outcome.results));
        if let Some(report) = outcome.ann_report {
            if let Some(rollout_policy) = rollout_policy {
                lines.push(crate::cli_ann::format_no_fallback_decision(
                    &evaluate_hnsw_no_fallback_rollout(rollout_policy, search_policy, &report),
                ));
            }
            lines.push(format_ann_search_report(&report));
        }
        Ok(lines.join("\n"))
    }
}

pub fn hnsw_no_fallback_profile_show(path: &str, json: bool) -> Result<String, String> {
    let db = open_database(path, false)?;
    let policy = db.hnsw_no_fallback_rollout_policy();
    if json {
        return Ok(no_fallback_profile_to_json(policy));
    }
    Ok(format_no_fallback_profile(policy))
}

pub fn hnsw_no_fallback_profile_set(
    path: &str,
    policy: HnswNoFallbackRolloutPolicy,
    json: bool,
) -> Result<String, String> {
    let mut db = open_database(path, false)?;
    db.set_hnsw_no_fallback_rollout_policy(policy)
        .map_err(fmt_engine_error)?;
    if json {
        return Ok(no_fallback_profile_to_json(Some(policy)));
    }
    Ok(format!(
        "hnsw_no_fallback_profile set\n{}",
        format_no_fallback_profile(Some(policy))
    ))
}

pub fn hnsw_no_fallback_profile_clear(path: &str, json: bool) -> Result<String, String> {
    let mut db = open_database(path, false)?;
    db.clear_hnsw_no_fallback_rollout_policy()
        .map_err(fmt_engine_error)?;
    if json {
        return Ok(no_fallback_profile_to_json(None));
    }
    Ok("hnsw_no_fallback_profile cleared".to_owned())
}

pub(crate) fn resolve_no_fallback_profile(
    db: &Database,
    explicit: Option<HnswNoFallbackRolloutPolicy>,
    use_profile: bool,
) -> Result<Option<HnswNoFallbackRolloutPolicy>, String> {
    if explicit.is_some() && use_profile {
        return Err("use either --no-fallback-rollout or --use-no-fallback-profile".to_owned());
    }
    if !use_profile {
        return Ok(explicit);
    }
    db.hnsw_no_fallback_rollout_policy()
        .map(Some)
        .ok_or_else(|| "no persisted HNSW no-fallback profile is configured".to_owned())
}

fn format_no_fallback_profile(policy: Option<HnswNoFallbackRolloutPolicy>) -> String {
    match policy {
        Some(policy) => format!(
            "hnsw_no_fallback_profile configured=true rollout_enabled={} min_recall_q16={} require_upper_layers={}",
            policy.rollout_enabled, policy.min_recall_q16, policy.require_upper_layers
        ),
        None => "hnsw_no_fallback_profile configured=false".to_owned(),
    }
}

fn format_ann_search_report(report: &cortex_engine::AnnSearchReport) -> String {
    let fallback_reason = report
        .fallback_reason
        .map(|reason| reason.as_str())
        .unwrap_or("none");
    let returned = match report.path {
        AnnSearchPath::HnswGraph => "hnsw_graph",
        AnnSearchPath::ExactFallback => "exact_fallback",
    };
    let recall = report
        .recall_q16
        .map(|value| value.to_string())
        .unwrap_or_else(|| "none".to_owned());
    let min_recall = report
        .min_recall_q16
        .map(|value| value.to_string())
        .unwrap_or_else(|| "none".to_owned());
    let visited = report.visited_candidates;
    let max_visited = report
        .max_visited_candidates
        .map_or_else(|| "none".to_owned(), |value| value.to_string());
    let violations = if report.slo_violations.is_empty() {
        "none".to_owned()
    } else {
        report
            .slo_violations
            .iter()
            .map(|violation| violation.as_str())
            .collect::<Vec<_>>()
            .join(",")
    };

    format!(
        "ann_path={returned} fallback_reason={fallback_reason} fallback_performed={} recall_q16={recall} min_recall_q16={min_recall} allowed_candidates={} visited_candidates={visited} max_visited_candidates={max_visited} hnsw_ef_construction={} require_slo={} production_safe={} slo_violations={violations}",
        report.fallback_performed,
        report.allowed_candidates,
        report.hnsw_ef_construction,
        report.require_slo,
        report.production_safe
    )
}
