use super::{
    classify_enterprise_rag_question, classify_enterprise_rag_question_type,
    EnterpriseRagQuestionType,
};

#[test]
fn classifies_enterprise_rag_types_from_question_text_only() {
    let cases = [
            (
                "What is the name of the new metric for streaming sessions?",
                EnterpriseRagQuestionType::Basic,
            ),
            (
                "How should I structure a prompt experiment to reduce overconfident mistakes?",
                EnterpriseRagQuestionType::Semantic,
            ),
            (
                "During the capacity incident, what two thresholds should trigger detection and how long were SLA targets breached?",
                EnterpriseRagQuestionType::IntraDocumentReasoning,
            ),
            (
                "For the Proxima Bank 429 spike, what caused throttling and how do we verify it is not burning route SLOs?",
                EnterpriseRagQuestionType::ProjectRelated,
            ),
            (
                "Who owns the launch blocker and what is the slipped rollout deadline?",
                EnterpriseRagQuestionType::ProjectRelated,
            ),
            (
                "In the March 2026 incident, what was the root cause and what immediate mitigation did SRE apply?",
                EnterpriseRagQuestionType::Constrained,
            ),
            (
                "What are the v2 score ranges and what were the previous thresholds?",
                EnterpriseRagQuestionType::ConflictingInfo,
            ),
            (
                "What is Redwood's end-to-end process for rotating production secrets?",
                EnterpriseRagQuestionType::Completeness,
            ),
            (
                "What is Redwood Inference's mission statement?",
                EnterpriseRagQuestionType::HighLevel,
            ),
            (
                "What exact queue token format and signing algorithm are configured in production?",
                EnterpriseRagQuestionType::InfoNotFound,
            ),
            (
                "When is the office refrigerator deep cleaning scheduled?",
                EnterpriseRagQuestionType::Miscellaneous,
            ),
        ];

    for (query, expected) in cases {
        assert_eq!(classify_enterprise_rag_question_type(query), expected);
    }
}

#[test]
fn classification_reports_confidence_and_signals() {
    let classified = classify_enterprise_rag_question(
        "What exact queue token format and signing algorithm are configured in production?",
    );

    assert_eq!(
        classified.question_type,
        EnterpriseRagQuestionType::InfoNotFound
    );
    assert!(classified.confidence_q16 > 32_768);
    assert!(!classified.matched_signals.is_empty());
}

#[test]
fn parses_public_enterprise_rag_type_labels() {
    assert_eq!(
        EnterpriseRagQuestionType::parse("intra_document_reasoning"),
        Some(EnterpriseRagQuestionType::IntraDocumentReasoning)
    );
    assert_eq!(
        EnterpriseRagQuestionType::parse("null_query"),
        Some(EnterpriseRagQuestionType::InfoNotFound)
    );
    assert_eq!(EnterpriseRagQuestionType::parse("unknown"), None);
}
