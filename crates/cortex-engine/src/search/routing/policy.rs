use crate::search::intent::{classify_enterprise_rag_question_type, EnterpriseRagQuestionType};

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

pub(super) fn policy_for_intent(intent: SearchQueryIntent) -> SearchRoutePolicy {
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
