use super::super::scores::CategoryScores;
use super::super::types::EnterpriseRagQuestionType;
use super::super::utils::contains_any;

pub(super) fn score_high_level(query: &str, scores: &mut CategoryScores) {
    if contains_any(
        query,
        &[
            "mission statement",
            "company's thesis",
            "competitive advantage",
            "business model",
            "revenue streams",
            "add-on categories",
            "major departments",
            "high-level organization",
            "stated differentiation",
            "policy dimensions",
        ],
    ) {
        scores.add(EnterpriseRagQuestionType::HighLevel, 8, "company_overview");
    }
    if contains_any(
        query,
        &[
            "high level",
            "big picture",
            "company overview",
            "features are highlighted",
            "optimizations are explicitly called out",
        ],
    ) {
        scores.add(EnterpriseRagQuestionType::HighLevel, 5, "overview_phrase");
    }
}

pub(super) fn score_miscellaneous(query: &str, scores: &mut CategoryScores) {
    if contains_any(
        query,
        &[
            "jokes",
            "memes",
            "refrigerator",
            "fridge",
            "office kitchen",
            "softball",
            "fantasy football",
            "sneakers",
            "cleats",
            "planter",
            "wall mounted art",
            "snake plants",
            "zz plants",
            "demo slides",
            "placeholder image",
            "go-to-market misc",
            "deep cleaning",
            "office summer",
            "interview",
        ],
    ) {
        scores.add(
            EnterpriseRagQuestionType::Miscellaneous,
            8,
            "non_product_topic",
        );
    }
    if contains_any(
        query,
        &["team chat about posting jokes", "4th floor office"],
    ) {
        scores.add(
            EnterpriseRagQuestionType::Miscellaneous,
            4,
            "office_or_chat_context",
        );
    }
}

pub(super) fn score_info_not_found(query: &str, scores: &mut CategoryScores) {
    if contains_any(
        query,
        &[
            "specific enterprise accounts",
            "initial allowlist",
            "exact per-route-group budget",
            "exact queue-depth",
            "complete mapping",
            "public blockchain",
            "smart contract address",
            "exact queue token",
            "per-request co2e",
            "bios settings",
            "safe mode boot",
            "cab approval",
            "quorum",
            "tie-break",
            "microsoft teams",
            "adaptive cards",
            "cryptographic signing algorithm",
            "key rotation cadence",
            "exact canonicalization rules",
            "full json schema",
            "checkpoint mismatch",
            "firmware rollout",
        ],
    ) {
        scores.add(
            EnterpriseRagQuestionType::InfoNotFound,
            8,
            "unavailable_specifics",
        );
    }
    if contains_any(
        query,
        &[
            "what exact",
            "required payload schema",
            "where is that mapping source-of-truth",
            "currently configured in production",
        ],
    ) {
        scores.add(
            EnterpriseRagQuestionType::InfoNotFound,
            3,
            "exact_internal_unknown",
        );
    }
}
