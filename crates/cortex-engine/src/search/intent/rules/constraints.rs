use super::super::scores::CategoryScores;
use super::super::types::EnterpriseRagQuestionType;
use super::super::utils::{contains_any, has_date_or_version_signal};

pub(super) fn score_completeness(query: &str, scores: &mut CategoryScores) {
    if contains_any(
        query,
        &[
            "procedure",
            "end-to-end process",
            "complete go/no-go gate",
            "across all",
            "across redwood's",
            "which sdk has the highest",
            "how many weekly",
            "most published customer stories",
            "most follow-up action items",
            "most production incidents",
            "which intake channel has the most",
            "has any customer other than",
            "how many fireflies",
            "list all",
            "list each",
            "comprehensive",
        ],
    ) {
        scores.add(
            EnterpriseRagQuestionType::Completeness,
            7,
            "multi_document_aggregate",
        );
    }
    if contains_any(
        query,
        &["list all", "list each", "complete", "comprehensive"],
    ) {
        scores.add(
            EnterpriseRagQuestionType::Completeness,
            4,
            "explicit_completeness_request",
        );
    }
    if contains_any(
        query,
        &[
            "required validations",
            "required approvals",
            "required customer communications",
            "including emergency",
        ],
    ) {
        scores.add(
            EnterpriseRagQuestionType::Completeness,
            3,
            "checklist_requirements",
        );
    }
}

pub(super) fn score_conflicting_info(query: &str, scores: &mut CategoryScores) {
    if contains_any(
        query,
        &[
            "latest baseline",
            "previous thresholds",
            "earlier %",
            "compared to",
            "was the degraded",
            "oom or intermittent",
            "manager or cost-ops",
            "latest baseline/growth/peak",
            "customer-managed kms",
            "hosted aws marketplace sku",
            "default ttl",
            "what % of interactive",
        ],
    ) {
        scores.add(
            EnterpriseRagQuestionType::ConflictingInfo,
            7,
            "version_or_option_conflict",
        );
    }
    if contains_any(
        query,
        &[
            "conflict",
            "conflicting",
            "contradict",
            "discrepancy",
            "changed from",
            "changed between",
            "previous",
            "earlier",
            "latest",
        ],
    ) {
        scores.add(
            EnterpriseRagQuestionType::ConflictingInfo,
            3,
            "conflict_vocabulary",
        );
    }
}

pub(super) fn score_constrained(query: &str, scores: &mut CategoryScores) {
    if has_date_or_version_signal(query) {
        scores.add(EnterpriseRagQuestionType::Constrained, 3, "date_or_version");
    }
    if contains_any(
        query,
        &[
            "incident where",
            "incident that",
            "root cause and what mitigation",
            "what caused the production",
            "underlying cause and what hotfix",
            "server-side mitigation",
            "immediate mitigation",
            "target ship date",
            "follow-up ticket",
            "controlled failover game day",
            "private/vpc deployment",
            "hosted api issues",
            "long-lived server-sent events",
        ],
    ) {
        scores.add(
            EnterpriseRagQuestionType::Constrained,
            6,
            "incident_constraint",
        );
    }
}
