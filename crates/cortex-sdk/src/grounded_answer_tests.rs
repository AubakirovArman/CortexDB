use crate::{
    AnswerGroundingOptionsResponse, AqlRetrievalMode, ContextPackCellResponse, ContextPackResponse,
    GroundedAnswerRequest, GroundedAnswerResponse,
};

fn test_context_pack() -> ContextPackResponse {
    ContextPackResponse {
        schema_version: ContextPackResponse::SCHEMA_VERSION_V1.to_owned(),
        token_budget_tokens: 256,
        estimated_tokens: 40,
        truncated: false,
        citations_required: true,
        answerability_q16: u16::MAX,
        conflict_visibility_q16: 0,
        visible_conflict_count: 0,
        cells: vec![ContextPackCellResponse {
            cell_id: 7,
            estimated_tokens: 40,
            citation: Some("doc://project-risk#p1".to_owned()),
            payload_text: "The migration blocker is the audit export dependency.".to_owned(),
            explain: None,
            source_ref: None,
            provenance: None,
            access_decision: None,
        }],
        anomalies: Vec::new(),
    }
}

#[test]
fn grounded_answer_request_builds_retrieve_and_verify_statements() {
    let request = GroundedAnswerRequest::new("project:alpha", "default", "migration blocker")
        .mode(AqlRetrievalMode::Balanced)
        .budget_tokens(512)
        .limit_candidates(12)
        .where_clause(r#"space = project:alpha AND status = "ready""#)
        .require_citations(true);

    assert_eq!(
        request.retrieve_statement().unwrap(),
        concat!(
            "RETRIEVE CONTEXT FOR TASK \"migration blocker\" IN BRAIN default ",
            "USING MODE balanced BUDGET 512 TOKENS LIMIT 12 CANDIDATES ",
            "WHERE space = project:alpha AND status = \"ready\" REQUIRE citations;"
        )
    );
    assert_eq!(
        request.verify_statement("The migration blocker is the audit export dependency.").unwrap(),
        Some(
            "VERIFY FACT \"The migration blocker is the audit export dependency.\" IN BRAIN default;"
                .to_owned()
        )
    );
}

#[test]
fn grounded_answer_response_collects_citations_and_grounding() {
    let request = GroundedAnswerRequest::new("project:alpha", "default", "migration blocker")
        .grounding_options(AnswerGroundingOptionsResponse {
            require_citations: true,
            reject_unsupported: true,
            ..AnswerGroundingOptionsResponse::default()
        });
    let response = GroundedAnswerResponse::from_context_answer(
        &request,
        request.retrieve_statement().unwrap(),
        request
            .verify_statement("The migration blocker is the audit export dependency.")
            .unwrap(),
        test_context_pack(),
        "The migration blocker is the audit export dependency.".to_owned(),
        None,
    );

    assert!(response.answer_supported());
    assert!(!response.rejected);
    assert_eq!(response.citations, vec!["doc://project-risk#p1"]);
    assert_eq!(response.used_context_cell_ids, vec![7]);
    assert_eq!(
        response.verify_statement.as_deref(),
        Some(
            "VERIFY FACT \"The migration blocker is the audit export dependency.\" IN BRAIN default;"
        )
    );
}

#[test]
fn grounded_answer_can_disable_verify_for_draft_answers() {
    let request = GroundedAnswerRequest::new("project:alpha", "default", "migration blocker")
        .verify_answer(false);

    assert_eq!(
        request
            .verify_statement("The migration blocker is the audit export dependency.")
            .unwrap(),
        None
    );
}
