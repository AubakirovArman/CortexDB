#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SearchRouteStrategy {
    Keyword,
    VectorAnn,
    VectorExact,
    Hybrid,
}

impl SearchRouteStrategy {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Keyword => "keyword",
            Self::VectorAnn => "vector_ann",
            Self::VectorExact => "vector_exact",
            Self::Hybrid => "hybrid",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SearchRouteDecision {
    pub requested_mode: String,
    pub selected_strategy: SearchRouteStrategy,
    pub reason: &'static str,
    pub text_available: bool,
    pub vector_available: bool,
}

impl SearchRouteDecision {
    pub fn search_mode(&self) -> &'static str {
        self.selected_strategy.as_str()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SearchRouteInput<'a> {
    pub requested_mode: &'a str,
    pub algorithm: &'a str,
    pub text_available: bool,
    pub vector_available: bool,
}

pub fn route_search_query(input: SearchRouteInput<'_>) -> Result<SearchRouteDecision, String> {
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
    Ok(SearchRouteDecision {
        requested_mode: input.requested_mode.to_owned(),
        selected_strategy,
        reason: route_reason(input, selected_strategy),
        text_available: input.text_available,
        vector_available: input.vector_available,
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

#[cfg(test)]
mod tests {
    use super::{route_search_query, SearchRouteInput, SearchRouteStrategy};

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
}
