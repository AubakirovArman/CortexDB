//! Generated from `crates/cortex-engine/fixtures/ranking_frozen_weights_v1.json`.
//! Keep behavior-identical ranking changes behind the frozen artifact gate.

#[allow(dead_code)]
pub const VERSION: &str = "ranking-frozen-weights-v1";
pub const Q16_ONE_U32: u32 = 65_535;
pub const Q16_ONE_U64: u64 = 65_535;
pub const Q16_SCALE_U32: u32 = 65_536;
pub const Q16_SCALE_I128: i128 = 65_536;

pub const CONTEXT_REDUNDANCY_PENALTY_WEIGHT: u32 = 10_000;

pub const VALUE_PER_TOKEN_COVERAGE_VALUE: i64 = 100_000;
pub const VALUE_PER_TOKEN_MATCHED_TERM_VALUE: i64 = 10_000;
pub const VALUE_PER_TOKEN_CITATION_VALUE: i64 = 5_000;
pub const VALUE_PER_TOKEN_REDUNDANCY_VALUE: i64 = 20_000;

pub const QUERY_BASE_TERM_WEIGHT: u32 = 4;
pub const QUERY_EXPANSION_WEIGHT: u32 = 1;
pub const QUERY_PHRASE_EXPANSION_WEIGHT: u32 = 2;
pub const QUERY_ANCHOR_STRONG_WEIGHT: u32 = 12;
pub const QUERY_ANCHOR_PHRASE_WEIGHT: u32 = 8;
pub const QUERY_ANCHOR_NUMERIC_WEIGHT: u32 = 6;

pub const RRF_RANK_CONSTANT: u64 = 60;
pub const RRF_SCORE_SCALE: u64 = 1_000_000;
pub const RRF_LEXICAL_HEAVY_Q16: u32 = 45_000;
pub const RRF_VECTOR_HEAVY_Q16: u32 = 20_000;
pub const RRF_BALANCED_LEXICAL_Q16: u32 = 32_768;
pub const RERANK_MIN_CANDIDATE_LIMIT: usize = 32;

pub const RERANK_DEFAULT_LEXICAL_WEIGHT: u32 = 2;
pub const RERANK_DEFAULT_VECTOR_WEIGHT: u32 = 2;
pub const RERANK_DEFAULT_ANCHOR_PAYLOAD_BONUS: u64 = 25_000;
pub const RERANK_DEFAULT_SOURCE_HINT_PAYLOAD_BONUS: u64 = 10_000;
pub const RERANK_DEFAULT_SCOPE_MAPPING_METADATA_BONUS: u64 = 1;
pub const RERANK_DEFAULT_CONDITION_PAYLOAD_BONUS: u64 = 1;
pub const RERANK_DEFAULT_NO_EVIDENCE_OVERLAP_Q16: u16 = 16_384;
pub const RERANK_REQUIREMENT_PAYLOAD_BONUS: u64 = 2_500;
pub const RERANK_TERM_PAYLOAD_BONUS: u64 = 1_000;
pub const EVIDENCE_OVERLAP_THRESHOLD: u32 = 2;
pub const EVIDENCE_ANCHOR_POINTS: u32 = 2;
pub const EVIDENCE_SOURCE_POINTS: u32 = 2;
pub const EVIDENCE_CONDITION_POINTS: u32 = 2;
pub const EVIDENCE_REQUIREMENT_POINTS: u32 = 2;
pub const EVIDENCE_TERM_POINTS: u32 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FrozenRerankProfile {
    pub lexical_weight: u32,
    pub vector_weight: u32,
    pub anchor_payload_bonus: u64,
    pub source_hint_payload_bonus: u64,
    pub scope_mapping_metadata_bonus: u64,
    pub condition_payload_bonus: u64,
    pub no_evidence_overlap_score_q16: u16,
    pub rrf_lexical_q16: u32,
}

pub const BASIC_PROFILE: FrozenRerankProfile = FrozenRerankProfile {
    lexical_weight: 3,
    vector_weight: 2,
    anchor_payload_bonus: RERANK_DEFAULT_ANCHOR_PAYLOAD_BONUS,
    source_hint_payload_bonus: RERANK_DEFAULT_SOURCE_HINT_PAYLOAD_BONUS,
    scope_mapping_metadata_bonus: RERANK_DEFAULT_SCOPE_MAPPING_METADATA_BONUS,
    condition_payload_bonus: RERANK_DEFAULT_CONDITION_PAYLOAD_BONUS,
    no_evidence_overlap_score_q16: 12_000,
    rrf_lexical_q16: RRF_LEXICAL_HEAVY_Q16,
};

pub const SEMANTIC_PROFILE: FrozenRerankProfile = FrozenRerankProfile {
    lexical_weight: 1,
    vector_weight: 4,
    anchor_payload_bonus: 18_000,
    source_hint_payload_bonus: RERANK_DEFAULT_SOURCE_HINT_PAYLOAD_BONUS,
    scope_mapping_metadata_bonus: RERANK_DEFAULT_SCOPE_MAPPING_METADATA_BONUS,
    condition_payload_bonus: RERANK_DEFAULT_CONDITION_PAYLOAD_BONUS,
    no_evidence_overlap_score_q16: 20_000,
    rrf_lexical_q16: RRF_VECTOR_HEAVY_Q16,
};

pub const PROJECT_RELATED_PROFILE: FrozenRerankProfile = FrozenRerankProfile {
    lexical_weight: 1,
    vector_weight: 4,
    anchor_payload_bonus: 30_000,
    source_hint_payload_bonus: RERANK_DEFAULT_SOURCE_HINT_PAYLOAD_BONUS,
    scope_mapping_metadata_bonus: 2,
    condition_payload_bonus: RERANK_DEFAULT_CONDITION_PAYLOAD_BONUS,
    no_evidence_overlap_score_q16: RERANK_DEFAULT_NO_EVIDENCE_OVERLAP_Q16,
    rrf_lexical_q16: 24_000,
};

pub const CONSTRAINED_PROFILE: FrozenRerankProfile = FrozenRerankProfile {
    lexical_weight: 4,
    vector_weight: 1,
    anchor_payload_bonus: 28_000,
    source_hint_payload_bonus: RERANK_DEFAULT_SOURCE_HINT_PAYLOAD_BONUS,
    scope_mapping_metadata_bonus: RERANK_DEFAULT_SCOPE_MAPPING_METADATA_BONUS,
    condition_payload_bonus: 3,
    no_evidence_overlap_score_q16: 10_000,
    rrf_lexical_q16: RRF_LEXICAL_HEAVY_Q16,
};

pub const CONFLICTING_INFO_PROFILE: FrozenRerankProfile = FrozenRerankProfile {
    lexical_weight: 3,
    vector_weight: 2,
    anchor_payload_bonus: RERANK_DEFAULT_ANCHOR_PAYLOAD_BONUS,
    source_hint_payload_bonus: 14_000,
    scope_mapping_metadata_bonus: RERANK_DEFAULT_SCOPE_MAPPING_METADATA_BONUS,
    condition_payload_bonus: RERANK_DEFAULT_CONDITION_PAYLOAD_BONUS,
    no_evidence_overlap_score_q16: RERANK_DEFAULT_NO_EVIDENCE_OVERLAP_Q16,
    rrf_lexical_q16: 38_000,
};

pub const COMPLETENESS_PROFILE: FrozenRerankProfile = FrozenRerankProfile {
    lexical_weight: 1,
    vector_weight: 4,
    anchor_payload_bonus: 22_000,
    source_hint_payload_bonus: RERANK_DEFAULT_SOURCE_HINT_PAYLOAD_BONUS,
    scope_mapping_metadata_bonus: RERANK_DEFAULT_SCOPE_MAPPING_METADATA_BONUS,
    condition_payload_bonus: RERANK_DEFAULT_CONDITION_PAYLOAD_BONUS,
    no_evidence_overlap_score_q16: 22_000,
    rrf_lexical_q16: 28_000,
};

pub const HIGH_LEVEL_PROFILE: FrozenRerankProfile = FrozenRerankProfile {
    lexical_weight: 1,
    vector_weight: 4,
    anchor_payload_bonus: RERANK_DEFAULT_ANCHOR_PAYLOAD_BONUS,
    source_hint_payload_bonus: 18_000,
    scope_mapping_metadata_bonus: RERANK_DEFAULT_SCOPE_MAPPING_METADATA_BONUS,
    condition_payload_bonus: RERANK_DEFAULT_CONDITION_PAYLOAD_BONUS,
    no_evidence_overlap_score_q16: 24_000,
    rrf_lexical_q16: RRF_VECTOR_HEAVY_Q16,
};

pub const INFO_NOT_FOUND_PROFILE: FrozenRerankProfile = FrozenRerankProfile {
    lexical_weight: 2,
    vector_weight: 2,
    anchor_payload_bonus: RERANK_DEFAULT_ANCHOR_PAYLOAD_BONUS,
    source_hint_payload_bonus: RERANK_DEFAULT_SOURCE_HINT_PAYLOAD_BONUS,
    scope_mapping_metadata_bonus: RERANK_DEFAULT_SCOPE_MAPPING_METADATA_BONUS,
    condition_payload_bonus: RERANK_DEFAULT_CONDITION_PAYLOAD_BONUS,
    no_evidence_overlap_score_q16: 8_000,
    rrf_lexical_q16: RRF_BALANCED_LEXICAL_Q16,
};

pub const METADATA_RERANK_SCALE: u64 = 1_024;
pub const METADATA_CONFLICTING_INFO_TRUST_Q16: u64 = 18_000;
pub const METADATA_CONFLICTING_INFO_FRESHNESS_Q16: u64 = 24_000;
pub const METADATA_CONSTRAINED_TEMPORAL_TRUST_Q16: u64 = 12_000;
pub const METADATA_CONSTRAINED_TEMPORAL_FRESHNESS_Q16: u64 = 22_000;
pub const METADATA_TEMPORAL_TRUST_Q16: u64 = 8_000;
pub const METADATA_TEMPORAL_FRESHNESS_Q16: u64 = 18_000;
pub const METADATA_INFO_NOT_FOUND_TRUST_Q16: u64 = 4_000;
pub const METADATA_INFO_NOT_FOUND_FRESHNESS_Q16: u64 = 2_000;
pub const METADATA_DEFAULT_TRUST_Q16: u64 = 3_000;
pub const METADATA_DEFAULT_FRESHNESS_Q16: u64 = 3_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FrozenRoutePolicy {
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

pub const LOOKUP_ROUTE_POLICY: FrozenRoutePolicy = FrozenRoutePolicy {
    candidate_limit_multiplier: 2,
    result_limit_cap: Some(5),
    token_budget_q16: 32_768,
    diversity_lambda_q16: 65_535,
    rerank: true,
    diversity: false,
    allow_abstain: false,
    lexical_weight_q16: 42_000,
    semantic_weight_q16: 18_000,
};

pub const SEMANTIC_ROUTE_POLICY: FrozenRoutePolicy = FrozenRoutePolicy {
    candidate_limit_multiplier: 5,
    result_limit_cap: None,
    token_budget_q16: 65_535,
    diversity_lambda_q16: 49_152,
    rerank: true,
    diversity: true,
    allow_abstain: false,
    lexical_weight_q16: 18_000,
    semantic_weight_q16: 42_000,
};

pub const PROJECT_RELATED_ROUTE_POLICY: FrozenRoutePolicy = FrozenRoutePolicy {
    candidate_limit_multiplier: 6,
    result_limit_cap: None,
    token_budget_q16: 65_535,
    diversity_lambda_q16: 52_428,
    rerank: true,
    diversity: true,
    allow_abstain: false,
    lexical_weight_q16: 28_000,
    semantic_weight_q16: 34_000,
};

pub const HIGH_LEVEL_ROUTE_POLICY: FrozenRoutePolicy = FrozenRoutePolicy {
    candidate_limit_multiplier: 8,
    result_limit_cap: None,
    token_budget_q16: 65_535,
    diversity_lambda_q16: 45_875,
    rerank: true,
    diversity: true,
    allow_abstain: false,
    lexical_weight_q16: 24_000,
    semantic_weight_q16: 36_000,
};

pub const CONFLICTING_INFO_ROUTE_POLICY: FrozenRoutePolicy = FrozenRoutePolicy {
    candidate_limit_multiplier: 6,
    result_limit_cap: None,
    token_budget_q16: 65_535,
    diversity_lambda_q16: 32_768,
    rerank: true,
    diversity: true,
    allow_abstain: false,
    lexical_weight_q16: 34_000,
    semantic_weight_q16: 26_000,
};

pub const COMPLETENESS_ROUTE_POLICY: FrozenRoutePolicy = FrozenRoutePolicy {
    candidate_limit_multiplier: 8,
    result_limit_cap: None,
    token_budget_q16: 65_535,
    diversity_lambda_q16: 36_864,
    rerank: true,
    diversity: true,
    allow_abstain: false,
    lexical_weight_q16: 30_000,
    semantic_weight_q16: 30_000,
};

pub const INFO_NOT_FOUND_ROUTE_POLICY: FrozenRoutePolicy = FrozenRoutePolicy {
    candidate_limit_multiplier: 3,
    result_limit_cap: Some(3),
    token_budget_q16: 24_576,
    diversity_lambda_q16: 65_535,
    rerank: true,
    diversity: false,
    allow_abstain: true,
    lexical_weight_q16: 36_000,
    semantic_weight_q16: 20_000,
};

pub const CONSTRAINED_ROUTE_POLICY: FrozenRoutePolicy = FrozenRoutePolicy {
    candidate_limit_multiplier: 4,
    result_limit_cap: Some(6),
    token_budget_q16: 49_152,
    diversity_lambda_q16: 65_535,
    rerank: true,
    diversity: false,
    allow_abstain: false,
    lexical_weight_q16: 38_000,
    semantic_weight_q16: 22_000,
};
