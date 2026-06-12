use super::super::scores::CategoryScores;
use super::super::types::EnterpriseRagQuestionType;
use super::super::utils::contains_any;

pub(super) fn score_semantic(query: &str, scores: &mut CategoryScores) {
    if contains_any(
        query,
        &[
            "when does booking open",
            "final concession terms",
            "low bit math",
            "what caused an enterprise tenant",
            "rollout system",
            "large overnight upload",
            "short-lived resume credential",
            "specific gate thresholds",
            "how should i structure",
            "staged rollout schedule",
            "temporary kill switch",
            "why would",
            "why does",
            "what happens when",
            "mandatory items",
            "advisory 0 to 100 score",
            "planned overnight time window",
            "storage setup and time-to-live",
            "internal admin ui",
            "internal routing performance memo",
            "what is the name of the new mechanism",
        ],
    ) {
        scores.add(
            EnterpriseRagQuestionType::Semantic,
            6,
            "semantic_paraphrase",
        );
    }
    if contains_any(
        query,
        &[
            "recommended",
            "recommend",
            "recommendation",
            "requirements for",
            "what caused",
            "how should i",
            "tracking things like",
            "which approach",
        ],
    ) {
        scores.add(EnterpriseRagQuestionType::Semantic, 2, "conceptual_query");
    }
}

pub(super) fn score_basic(query: &str, scores: &mut CategoryScores) {
    if contains_any(
        query,
        &[
            "what are the default",
            "what is the name",
            "acceptance criteria",
            "in the meeting",
            "draft spec",
            "internal shiproom",
            "what support response time",
            "what keyboard-only",
            "where was",
            "what mitigation was proposed",
            "how does the new alerting",
        ],
    ) {
        scores.add(EnterpriseRagQuestionType::Basic, 4, "known_item_lookup");
    }
}
