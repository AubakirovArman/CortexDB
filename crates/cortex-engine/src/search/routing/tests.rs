use super::{
    classify_search_query_intent, route_policy_for_query, route_search_query,
    route_search_query_for_text, routed_candidate_limit, routed_result_limit, routed_token_budget,
    SearchQueryIntent, SearchRouteInput, SearchRouteStrategy,
};

#[test]
fn explicit_keyword_routes_to_keyword() {
    let decision = route_search_query(SearchRouteInput {
        requested_mode: "keyword",
        algorithm: "ann",
        text_available: true,
        vector_available: false,
    })
    .unwrap();

    assert_eq!(decision.selected_strategy, SearchRouteStrategy::Keyword);
    assert_eq!(decision.search_mode(), "keyword");
    assert_eq!(decision.reason, "explicit_keyword_mode");
    assert_eq!(decision.intent, SearchQueryIntent::Lookup);
}

#[test]
fn explicit_vector_routes_by_algorithm() {
    let ann = route_search_query(SearchRouteInput {
        requested_mode: "vector",
        algorithm: "ann",
        text_available: false,
        vector_available: true,
    })
    .unwrap();
    let exact = route_search_query(SearchRouteInput {
        requested_mode: "vector",
        algorithm: "exact",
        text_available: false,
        vector_available: true,
    })
    .unwrap();

    assert_eq!(ann.selected_strategy, SearchRouteStrategy::VectorAnn);
    assert_eq!(ann.search_mode(), "vector_ann");
    assert_eq!(exact.selected_strategy, SearchRouteStrategy::VectorExact);
    assert_eq!(exact.search_mode(), "vector_exact");
}

#[test]
fn auto_routes_text_and_vector_to_hybrid() {
    let decision = route_search_query(SearchRouteInput {
        requested_mode: "auto",
        algorithm: "ann",
        text_available: true,
        vector_available: true,
    })
    .unwrap();

    assert_eq!(decision.selected_strategy, SearchRouteStrategy::Hybrid);
    assert_eq!(decision.reason, "auto_text_and_vector_available");
}

#[test]
fn auto_routes_vector_only_to_selected_vector_algorithm() {
    let decision = route_search_query(SearchRouteInput {
        requested_mode: "auto",
        algorithm: "exact",
        text_available: false,
        vector_available: true,
    })
    .unwrap();

    assert_eq!(decision.selected_strategy, SearchRouteStrategy::VectorExact);
    assert_eq!(decision.reason, "auto_vector_available_without_text");
}

#[test]
fn hybrid_requires_vector() {
    let error = route_search_query(SearchRouteInput {
        requested_mode: "hybrid",
        algorithm: "ann",
        text_available: true,
        vector_available: false,
    })
    .unwrap_err();

    assert_eq!(error, "mode=hybrid requires vector=<i16,...>");
}

#[test]
fn invalid_mode_and_algorithm_fail_closed() {
    let invalid_mode = route_search_query(SearchRouteInput {
        requested_mode: "semantic",
        algorithm: "ann",
        text_available: true,
        vector_available: false,
    })
    .unwrap_err();
    let invalid_algorithm = route_search_query(SearchRouteInput {
        requested_mode: "vector",
        algorithm: "flat",
        text_available: false,
        vector_available: true,
    })
    .unwrap_err();

    assert_eq!(
        invalid_mode,
        "mode must be keyword, vector, hybrid, or auto"
    );
    assert_eq!(invalid_algorithm, "algorithm must be exact or ann");
}

#[test]
fn classifies_query_intent_from_text_without_oracle_labels() {
    assert_eq!(
        classify_search_query_intent("Give me the high level company overview"),
        SearchQueryIntent::HighLevel
    );
    assert_eq!(
        classify_search_query_intent("List all blockers for the Apollo rollout"),
        SearchQueryIntent::Completeness
    );
    assert_eq!(
        classify_search_query_intent("Why did the team recommend SSO?"),
        SearchQueryIntent::Semantic
    );
}

#[test]
fn route_for_text_attaches_intent_policy_without_changing_explicit_strategy() {
    let decision = route_search_query_for_text(
        SearchRouteInput {
            requested_mode: "hybrid",
            algorithm: "ann",
            text_available: true,
            vector_available: true,
        },
        "Give me the high level company overview",
    )
    .unwrap();

    assert_eq!(decision.selected_strategy, SearchRouteStrategy::Hybrid);
    assert_eq!(decision.intent, SearchQueryIntent::HighLevel);
    assert!(decision.policy.diversity);
    assert_eq!(decision.policy.candidate_limit(10), 80);
}

#[test]
fn routed_candidate_limit_is_wider_for_completeness_questions() {
    assert_eq!(routed_candidate_limit("Find invoice Q4", 10), 20);
    assert_eq!(routed_candidate_limit("List all project blockers", 10), 80);
}

#[test]
fn routed_result_limit_and_token_budget_are_compact_for_lookup_only() {
    assert_eq!(routed_result_limit("Find invoice Q4", 10), 5);
    assert_eq!(routed_result_limit("List all project blockers", 10), 10);
    assert_eq!(routed_token_budget("Find invoice Q4", 1_000), 500);
    assert_eq!(
        routed_token_budget("List all project blockers", 1_000),
        1_000
    );
}

#[test]
fn diversity_lambda_is_stronger_for_coverage_and_conflict_queries() {
    let lookup = route_policy_for_query("Find invoice Q4");
    let semantic = route_policy_for_query("Why did the team recommend SSO?");
    let project = route_policy_for_query("Who owns the Apollo rollout blocker deadline status?");
    let completeness = route_policy_for_query("List all blockers for the Apollo rollout");
    let conflict = route_policy_for_query("What changed between the two policy versions?");

    assert!(!lookup.diversity);
    assert!(semantic.diversity);
    assert!(project.diversity);
    assert!(completeness.diversity);
    assert!(conflict.diversity);
    assert!(conflict.diversity_lambda_q16 < completeness.diversity_lambda_q16);
    assert!(completeness.diversity_lambda_q16 < semantic.diversity_lambda_q16);
    assert!(semantic.diversity_lambda_q16 < project.diversity_lambda_q16);
}
