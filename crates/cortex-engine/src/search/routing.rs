use super::intent::{classify_enterprise_rag_question_type, EnterpriseRagQuestionType};

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SearchQueryIntent {
    Lookup,
    Semantic,
    ProjectRelated,
    HighLevel,
    ConflictingInfo,
    Completeness,
    InfoNotFound,
    Constrained,
}

impl SearchQueryIntent {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Lookup => "lookup",
            Self::Semantic => "semantic",
            Self::ProjectRelated => "project_related",
            Self::HighLevel => "high_level",
            Self::ConflictingInfo => "conflicting_info",
            Self::Completeness => "completeness",
            Self::InfoNotFound => "info_not_found",
            Self::Constrained => "constrained",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SearchRoutePolicy {
    pub candidate_limit_multiplier: usize,
    pub result_limit_cap: Option<usize>,
    pub token_budget_q16: u16,
    pub diversity_lambda_q16: u16,
    pub rerank: bool,
    pub diversity: bool,
    pub allow_abstain: bool,
    pub lexical_weight_q16: u16,
    pub semantic_weight_q16: u16,
}

impl SearchRoutePolicy {
    pub fn candidate_limit(self, requested_limit: usize) -> usize {
        requested_limit.saturating_mul(self.candidate_limit_multiplier.max(1))
    }

    pub fn result_limit(self, requested_limit: usize) -> usize {
        if requested_limit == 0 {
            return 0;
        }
        self.result_limit_cap
            .map(|cap| requested_limit.min(cap.max(1)))
            .unwrap_or(requested_limit)
    }

    pub fn token_budget(self, requested_budget: u32) -> u32 {
        if requested_budget == 0 {
            return 0;
        }
        let adjusted =
            u64::from(requested_budget).saturating_mul(u64::from(self.token_budget_q16)) / 65_535;
        u32::try_from(adjusted.max(1)).unwrap_or(u32::MAX)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SearchRouteDecision {
    pub requested_mode: String,
    pub selected_strategy: SearchRouteStrategy,
    pub reason: &'static str,
    pub text_available: bool,
    pub vector_available: bool,
    pub intent: SearchQueryIntent,
    pub policy: SearchRoutePolicy,
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
    route_search_query_inner(input, None)
}

pub fn route_search_query_for_text(
    input: SearchRouteInput<'_>,
    query_text: &str,
) -> Result<SearchRouteDecision, String> {
    route_search_query_inner(input, Some(query_text))
}

pub fn classify_search_query_intent(query_text: &str) -> SearchQueryIntent {
    match classify_enterprise_rag_question_type(query_text) {
        EnterpriseRagQuestionType::Basic | EnterpriseRagQuestionType::Miscellaneous => {
            SearchQueryIntent::Lookup
        }
        EnterpriseRagQuestionType::Semantic | EnterpriseRagQuestionType::IntraDocumentReasoning => {
            SearchQueryIntent::Semantic
        }
        EnterpriseRagQuestionType::ProjectRelated => SearchQueryIntent::ProjectRelated,
        EnterpriseRagQuestionType::Constrained => SearchQueryIntent::Constrained,
        EnterpriseRagQuestionType::ConflictingInfo => SearchQueryIntent::ConflictingInfo,
        EnterpriseRagQuestionType::Completeness => SearchQueryIntent::Completeness,
        EnterpriseRagQuestionType::HighLevel => SearchQueryIntent::HighLevel,
        EnterpriseRagQuestionType::InfoNotFound => SearchQueryIntent::InfoNotFound,
    }
}

pub fn route_policy_for_query(query_text: &str) -> SearchRoutePolicy {
    policy_for_intent(classify_search_query_intent(query_text))
}

pub fn routed_candidate_limit(query_text: &str, requested_limit: usize) -> usize {
    route_policy_for_query(query_text).candidate_limit(requested_limit)
}

pub fn routed_result_limit(query_text: &str, requested_limit: usize) -> usize {
    route_policy_for_query(query_text).result_limit(requested_limit)
}

pub fn routed_token_budget(query_text: &str, requested_budget: u32) -> u32 {
    route_policy_for_query(query_text).token_budget(requested_budget)
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

fn policy_for_intent(intent: SearchQueryIntent) -> SearchRoutePolicy {
    match intent {
        SearchQueryIntent::Lookup => SearchRoutePolicy {
            candidate_limit_multiplier: 2,
            result_limit_cap: Some(5),
            token_budget_q16: 32_768,
            diversity_lambda_q16: 65_535,
            rerank: true,
            diversity: false,
            allow_abstain: false,
            lexical_weight_q16: 42_000,
            semantic_weight_q16: 18_000,
        },
        SearchQueryIntent::Semantic => SearchRoutePolicy {
            candidate_limit_multiplier: 5,
            result_limit_cap: None,
            token_budget_q16: 65_535,
            diversity_lambda_q16: 49_152,
            rerank: true,
            diversity: true,
            allow_abstain: false,
            lexical_weight_q16: 18_000,
            semantic_weight_q16: 42_000,
        },
        SearchQueryIntent::ProjectRelated => SearchRoutePolicy {
            candidate_limit_multiplier: 6,
            result_limit_cap: None,
            token_budget_q16: 65_535,
            diversity_lambda_q16: 52_428,
            rerank: true,
            diversity: true,
            allow_abstain: false,
            lexical_weight_q16: 28_000,
            semantic_weight_q16: 34_000,
        },
        SearchQueryIntent::HighLevel => SearchRoutePolicy {
            candidate_limit_multiplier: 8,
            result_limit_cap: None,
            token_budget_q16: 65_535,
            diversity_lambda_q16: 45_875,
            rerank: true,
            diversity: true,
            allow_abstain: false,
            lexical_weight_q16: 24_000,
            semantic_weight_q16: 36_000,
        },
        SearchQueryIntent::ConflictingInfo => SearchRoutePolicy {
            candidate_limit_multiplier: 6,
            result_limit_cap: None,
            token_budget_q16: 65_535,
            diversity_lambda_q16: 32_768,
            rerank: true,
            diversity: true,
            allow_abstain: false,
            lexical_weight_q16: 34_000,
            semantic_weight_q16: 26_000,
        },
        SearchQueryIntent::Completeness => SearchRoutePolicy {
            candidate_limit_multiplier: 8,
            result_limit_cap: None,
            token_budget_q16: 65_535,
            diversity_lambda_q16: 36_864,
            rerank: true,
            diversity: true,
            allow_abstain: false,
            lexical_weight_q16: 30_000,
            semantic_weight_q16: 30_000,
        },
        SearchQueryIntent::InfoNotFound => SearchRoutePolicy {
            candidate_limit_multiplier: 3,
            result_limit_cap: Some(3),
            token_budget_q16: 24_576,
            diversity_lambda_q16: 65_535,
            rerank: true,
            diversity: false,
            allow_abstain: true,
            lexical_weight_q16: 36_000,
            semantic_weight_q16: 20_000,
        },
        SearchQueryIntent::Constrained => SearchRoutePolicy {
            candidate_limit_multiplier: 4,
            result_limit_cap: Some(6),
            token_budget_q16: 49_152,
            diversity_lambda_q16: 65_535,
            rerank: true,
            diversity: false,
            allow_abstain: false,
            lexical_weight_q16: 38_000,
            semantic_weight_q16: 22_000,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::{
        classify_search_query_intent, route_policy_for_query, route_search_query,
        route_search_query_for_text, routed_candidate_limit, routed_result_limit,
        routed_token_budget, SearchQueryIntent, SearchRouteInput, SearchRouteStrategy,
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
        let project =
            route_policy_for_query("Who owns the Apollo rollout blocker deadline status?");
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
}
