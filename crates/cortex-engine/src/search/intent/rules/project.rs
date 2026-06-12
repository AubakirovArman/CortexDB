use super::super::scores::CategoryScores;
use super::super::types::EnterpriseRagQuestionType;
use super::super::utils::contains_any;

pub(super) fn score_project_related(query: &str, scores: &mut CategoryScores) {
    if contains_any(
        query,
        &[
            "support explain",
            "support handle",
            "sales share",
            "approved stance",
            "evidence pack",
            "deal",
            "credit requests",
            "policy we should follow",
            "what ui or api changes should we make",
            "how do we verify",
            "how should support",
            "how should sales",
            "explain and remediate",
            "enterprise ticket",
            "support bridge",
            "customer update cadence",
            "approvals are required",
            "issuing a credit",
            "standardizing on",
            "what is our approved stance",
            "what caused the savings",
            "do we need to recalculate",
            "when a tenant has",
            "what do we need to install before running restore",
        ],
    ) {
        scores.add(
            EnterpriseRagQuestionType::ProjectRelated,
            7,
            "customer_or_project_resolution",
        );
    }
    if contains_any(query, &["launch", "rollout", "release"])
        && contains_any(
            query,
            &[
                "owner", "owns", "dri", "blocker", "blocked", "deadline", "slipped", "status",
                "risk",
            ],
        )
    {
        scores.add(
            EnterpriseRagQuestionType::ProjectRelated,
            6,
            "project_delivery_status",
        );
    }
    if contains_any(
        query,
        &[
            "customer-facing",
            "customer-facing support",
            "enterprise route slo",
        ],
    ) {
        scores.add(
            EnterpriseRagQuestionType::ProjectRelated,
            2,
            "project_context_prefix",
        );
    }
}

pub(super) fn score_intra_document_reasoning(query: &str, scores: &mut CategoryScores) {
    if contains_any(
        query,
        &[
            "what two thresholds",
            "which meeting time",
            "48 hours after the call",
            "which teams are required to sign off",
            "what two follow-up",
            "at the start and what",
            "final tracked wording",
            "what singleton-wrapped",
            "what normalizer",
            "what alert triggered",
            "what two preventative follow-ups",
            "what model and metrics",
            "what artifacts",
            "hybrid approach",
            "30-minute follow-up call",
            "what support ticket number",
            "what request id",
            "if the target go-live",
            "what date should",
            "two unit primitives",
            " and where is the referenced ",
        ],
    ) {
        scores.add(
            EnterpriseRagQuestionType::IntraDocumentReasoning,
            7,
            "multi_fact_same_doc",
        );
    }
    if query.starts_with("during ") || query.starts_with("when using ") {
        scores.add(
            EnterpriseRagQuestionType::IntraDocumentReasoning,
            2,
            "same_document_context",
        );
    }
}
