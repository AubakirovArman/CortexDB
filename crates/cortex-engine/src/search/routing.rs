mod decision;
mod policy;
#[cfg(test)]
mod tests;

pub use decision::{SearchRouteDecision, SearchRouteInput, SearchRouteStrategy};
pub use policy::{
    classify_search_query_intent, route_policy_for_query, routed_candidate_limit,
    routed_result_limit, routed_token_budget, SearchQueryIntent, SearchRoutePolicy,
};

use policy::policy_for_intent;

pub fn route_search_query(input: SearchRouteInput<'_>) -> Result<SearchRouteDecision, String> {
    route_search_query_inner(input, None)
}

pub fn route_search_query_for_text(
    input: SearchRouteInput<'_>,
    query_text: &str,
) -> Result<SearchRouteDecision, String> {
    route_search_query_inner(input, Some(query_text))
}

fn route_search_query_inner(
    input: SearchRouteInput<'_>,
    query_text: Option<&str>,
) -> Result<SearchRouteDecision, String> {
    let selected_strategy = match input.requested_mode {
        "keyword" => SearchRouteStrategy::Keyword,
        "vector" => vector_strategy(input.algorithm)?,
        "hybrid" => {
            if !input.vector_available {
                return Err("mode=hybrid requires vector=<i16,...>".to_owned());
            }
            SearchRouteStrategy::Hybrid
        }
        "auto" => auto_strategy(input)?,
        _ => return Err("mode must be keyword, vector, hybrid, or auto".to_owned()),
    };
    let intent = query_text
        .map(classify_search_query_intent)
        .unwrap_or(SearchQueryIntent::Lookup);
    Ok(SearchRouteDecision {
        requested_mode: input.requested_mode.to_owned(),
        selected_strategy,
        reason: route_reason(input, selected_strategy),
        text_available: input.text_available,
        vector_available: input.vector_available,
        intent,
        policy: policy_for_intent(intent),
    })
}

fn vector_strategy(algorithm: &str) -> Result<SearchRouteStrategy, String> {
    match algorithm {
        "ann" => Ok(SearchRouteStrategy::VectorAnn),
        "exact" => Ok(SearchRouteStrategy::VectorExact),
        _ => Err("algorithm must be exact or ann".to_owned()),
    }
}

fn auto_strategy(input: SearchRouteInput<'_>) -> Result<SearchRouteStrategy, String> {
    match (input.text_available, input.vector_available) {
        (true, true) => Ok(SearchRouteStrategy::Hybrid),
        (false, true) => vector_strategy(input.algorithm),
        _ => Ok(SearchRouteStrategy::Keyword),
    }
}

fn route_reason(
    input: SearchRouteInput<'_>,
    selected_strategy: SearchRouteStrategy,
) -> &'static str {
    match input.requested_mode {
        "auto" if selected_strategy == SearchRouteStrategy::Hybrid => {
            "auto_text_and_vector_available"
        }
        "auto"
            if matches!(
                selected_strategy,
                SearchRouteStrategy::VectorAnn | SearchRouteStrategy::VectorExact
            ) =>
        {
            "auto_vector_available_without_text"
        }
        "auto" => "auto_text_only_or_default",
        "keyword" => "explicit_keyword_mode",
        "vector" => "explicit_vector_mode",
        "hybrid" => "explicit_hybrid_mode",
        _ => "unknown",
    }
}
