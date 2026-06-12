use cortex_engine::{
    evaluate_hnsw_no_fallback_rollout, AnnSearchPolicy, AnnSearchReport, HnswNoFallbackDecision,
    HnswNoFallbackRolloutPolicy, SearchRouteDecision,
};

use crate::responses::{
    AnnNoFallbackDecisionResponse, AnnSearchReportResponse, RouterError, SearchResponse,
    SearchResultResponse, SearchRoutingDecisionResponse,
};

pub(super) fn encode_response(
    search_mode: &str,
    routing: Option<SearchRouteDecision>,
    rerank: Option<String>,
    results: Vec<cortex_engine::DatabaseSearchResult>,
    ann_report: Option<AnnSearchReport>,
    no_fallback_decision: Option<AnnNoFallbackDecisionResponse>,
) -> Result<String, RouterError> {
    let response = SearchResponse {
        search_mode: search_mode.to_owned(),
        routing: routing.map(routing_response),
        rerank,
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

pub(super) fn routing_response(decision: SearchRouteDecision) -> SearchRoutingDecisionResponse {
    SearchRoutingDecisionResponse {
        requested_mode: decision.requested_mode,
        selected_strategy: decision.selected_strategy.as_str().to_owned(),
        reason: decision.reason.to_owned(),
        text_available: decision.text_available,
        vector_available: decision.vector_available,
    }
}

pub(super) fn report_response(report: AnnSearchReport) -> AnnSearchReportResponse {
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

pub(super) fn rollout_decision(
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
