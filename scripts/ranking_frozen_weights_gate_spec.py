"""Static paths and markers for the frozen ranking-weights gate."""

from __future__ import annotations

from pathlib import Path


SCHEMA_VERSION = "cortexdb.ranking_frozen_weights_check.v1"
FIXTURE_SCHEMA = "cortexdb.ranking_frozen_weights.v1"
FIXTURE_VERSION = "ranking-frozen-weights-v1"
MODULE_PATH = Path("crates/cortex-engine/src/search/frozen_weights.rs")

PROFILE_FIELDS = [
    "lexical_weight",
    "vector_weight",
    "anchor_payload_bonus",
    "source_hint_payload_bonus",
    "scope_mapping_metadata_bonus",
    "condition_payload_bonus",
    "no_evidence_overlap_score_q16",
    "rrf_lexical_q16",
]
PROFILE_CONSTS = {
    "basic": "BASIC_PROFILE",
    "semantic": "SEMANTIC_PROFILE",
    "project_related": "PROJECT_RELATED_PROFILE",
    "constrained": "CONSTRAINED_PROFILE",
    "conflicting_info": "CONFLICTING_INFO_PROFILE",
    "completeness": "COMPLETENESS_PROFILE",
    "high_level": "HIGH_LEVEL_PROFILE",
    "info_not_found": "INFO_NOT_FOUND_PROFILE",
}
ROUTE_FIELDS = [
    "candidate_limit_multiplier",
    "result_limit_cap",
    "token_budget_q16",
    "diversity_lambda_q16",
    "lexical_weight_q16",
    "semantic_weight_q16",
]
ROUTE_CONSTS = {
    "lookup": "LOOKUP_ROUTE_POLICY",
    "semantic": "SEMANTIC_ROUTE_POLICY",
    "project_related": "PROJECT_RELATED_ROUTE_POLICY",
    "high_level": "HIGH_LEVEL_ROUTE_POLICY",
    "conflicting_info": "CONFLICTING_INFO_ROUTE_POLICY",
    "completeness": "COMPLETENESS_ROUTE_POLICY",
    "info_not_found": "INFO_NOT_FOUND_ROUTE_POLICY",
    "constrained": "CONSTRAINED_ROUTE_POLICY",
}

CONSUMER_MARKERS = {
    "crates/cortex-engine/src/context/pack/builder.rs": [
        "frozen_weights::CONTEXT_REDUNDANCY_PENALTY_WEIGHT",
        "frozen_weights::Q16_SCALE_U32",
    ],
    "crates/cortex-engine/src/context/value_per_token.rs": [
        "frozen_weights::VALUE_PER_TOKEN_COVERAGE_VALUE",
        "frozen_weights::VALUE_PER_TOKEN_MATCHED_TERM_VALUE",
        "frozen_weights::VALUE_PER_TOKEN_CITATION_VALUE",
        "frozen_weights::VALUE_PER_TOKEN_REDUNDANCY_VALUE",
        "frozen_weights::Q16_SCALE_I128",
    ],
    "crates/cortex-engine/src/search/query_understanding.rs": [
        "frozen_weights::QUERY_BASE_TERM_WEIGHT",
        "frozen_weights::QUERY_ANCHOR_STRONG_WEIGHT",
    ],
    "crates/cortex-engine/src/search/indexes.rs": [
        "frozen_weights::RRF_SCORE_SCALE",
        "frozen_weights::RRF_RANK_CONSTANT",
        "frozen_weights::Q16_ONE_U64",
        "frozen_weights::RERANK_MIN_CANDIDATE_LIMIT",
    ],
    "crates/cortex-engine/src/search/database/ranking.rs": [
        "frozen_weights::METADATA_CONFLICTING_INFO_TRUST_Q16",
        "frozen_weights::METADATA_RERANK_SCALE",
        "frozen_weights::RERANK_MIN_CANDIDATE_LIMIT",
    ],
    "crates/cortex-engine/src/search/rerank/types.rs": [
        "frozen_weights::RRF_LEXICAL_HEAVY_Q16",
        "frozen_weights::RERANK_DEFAULT_ANCHOR_PAYLOAD_BONUS",
        "frozen_weights::RERANK_DEFAULT_NO_EVIDENCE_OVERLAP_Q16",
    ],
    "crates/cortex-engine/src/search/rerank/scoring.rs": [
        "frozen_weights::RERANK_REQUIREMENT_PAYLOAD_BONUS",
        "frozen_weights::RERANK_TERM_PAYLOAD_BONUS",
        "frozen_weights::EVIDENCE_OVERLAP_THRESHOLD",
    ],
    "crates/cortex-engine/src/search/rerank/calibration.rs": [
        "FrozenRerankProfile",
        "frozen_weights::BASIC_PROFILE",
        "frozen_weights::CONFLICTING_INFO_PROFILE",
    ],
    "crates/cortex-engine/src/search/routing/policy.rs": [
        "FrozenRoutePolicy",
        "frozen_weights::LOOKUP_ROUTE_POLICY",
        "frozen_weights::INFO_NOT_FOUND_ROUTE_POLICY",
    ],
}

BANNED_MARKERS = {
    "crates/cortex-engine/src/context/pack/builder.rs": [
        "* 10_000) / 65536",
        "* 10000) / 65536",
    ],
    "crates/cortex-engine/src/context/value_per_token.rs": [
        "const COVERAGE_VALUE",
        "const MATCHED_TERM_VALUE",
        "const CITATION_VALUE",
        "const REDUNDANCY_VALUE",
        "saturating_mul(65_536)",
        "/ 65_536",
    ],
    "crates/cortex-engine/src/search/query_understanding.rs": [
        "weight: 4",
        "weight: 12",
    ],
    "crates/cortex-engine/src/search/indexes.rs": [
        "1_000_000",
        "60 +",
        "/ 65_535",
        ".max(32)",
    ],
    "crates/cortex-engine/src/search/database/ranking.rs": [
        "u64::from(u16::MAX)",
        ".max(32)",
        "1_024",
    ],
    "crates/cortex-engine/src/search/rerank/types.rs": [
        "45_000",
        "20_000",
        "32_768",
        "25_000",
        "16_384",
    ],
    "crates/cortex-engine/src/search/rerank/scoring.rs": [
        "const REQUIREMENT_PAYLOAD_BONUS",
        "2_500",
        "1_000",
        "65_535",
    ],
    "crates/cortex-engine/src/search/rerank/calibration.rs": [
        "12_000",
        "18_000",
        "30_000",
        "38_000",
    ],
    "crates/cortex-engine/src/search/routing/policy.rs": [
        "42_000",
        "49_152",
        "24_576",
        "65_535",
    ],
}
