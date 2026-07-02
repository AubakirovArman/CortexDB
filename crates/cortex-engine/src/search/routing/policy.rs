use crate::search::intent::{classify_enterprise_rag_question_type, EnterpriseRagQuestionType};

use super::super::frozen_weights::{self, FrozenRoutePolicy};

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
        let adjusted = u64::from(requested_budget).saturating_mul(u64::from(self.token_budget_q16))
            / frozen_weights::Q16_ONE_U64;
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
    let frozen = match intent {
        SearchQueryIntent::Lookup => frozen_weights::LOOKUP_ROUTE_POLICY,
        SearchQueryIntent::Semantic => frozen_weights::SEMANTIC_ROUTE_POLICY,
        SearchQueryIntent::ProjectRelated => frozen_weights::PROJECT_RELATED_ROUTE_POLICY,
        SearchQueryIntent::HighLevel => frozen_weights::HIGH_LEVEL_ROUTE_POLICY,
        SearchQueryIntent::ConflictingInfo => frozen_weights::CONFLICTING_INFO_ROUTE_POLICY,
        SearchQueryIntent::Completeness => frozen_weights::COMPLETENESS_ROUTE_POLICY,
        SearchQueryIntent::InfoNotFound => frozen_weights::INFO_NOT_FOUND_ROUTE_POLICY,
        SearchQueryIntent::Constrained => frozen_weights::CONSTRAINED_ROUTE_POLICY,
    };
    route_policy_from_frozen(frozen)
}

fn route_policy_from_frozen(frozen: FrozenRoutePolicy) -> SearchRoutePolicy {
    SearchRoutePolicy {
        candidate_limit_multiplier: frozen.candidate_limit_multiplier,
        result_limit_cap: frozen.result_limit_cap,
        token_budget_q16: frozen.token_budget_q16,
        diversity_lambda_q16: frozen.diversity_lambda_q16,
        rerank: frozen.rerank,
        diversity: frozen.diversity,
        allow_abstain: frozen.allow_abstain,
        lexical_weight_q16: frozen.lexical_weight_q16,
        semantic_weight_q16: frozen.semantic_weight_q16,
    }
}
