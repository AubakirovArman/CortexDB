use crate::{
    AnswerGroundingOptionsV1, ContextPackAccessDecisionV1, ContextPackAnomalyV1, ContextPackCellV1,
    ContextPackExplainV1, ContextPackProvenanceV1, ContextPackSourceRefV1, ContextPackV1,
    ScoreComponentResponse,
};

#[test]
fn context_pack_v1_sdk_models_roundtrip_full_shape() {
    let pack = ContextPackV1 {
        schema_version: ContextPackV1::SCHEMA_VERSION_V1.to_owned(),
        token_budget_tokens: 128,
        estimated_tokens: 92,
        truncated: false,
        citations_required: true,
        answerability_q16: 60_000,
        conflict_visibility_q16: 1_024,
        visible_conflict_count: 1,
        cells: vec![ContextPackCellV1 {
            cell_id: 42,
            estimated_tokens: 45,
            citation: Some("ifc:project-42".to_owned()),
            payload_text: "scope=project:investments\nstatus=ready\nbody".to_owned(),
            explain: Some(ContextPackExplainV1 {
                score: 120,
                matched_terms: vec!["budget".to_owned(), "solar".to_owned()],
                why_selected: "matched required project terms".to_owned(),
                score_components: vec![ScoreComponentResponse {
                    name: "source_trust".to_owned(),
                    value: 60_000,
                    contribution: 8,
                    reason: "official_source".to_owned(),
                }],
                base_bm25: 90,
                source_trust_q16: 60_000,
                source_trust_category: "official".to_owned(),
                source_trust_bonus: 8,
                source_freshness_q16: 65_535,
                source_freshness_category: "current".to_owned(),
                source_freshness_bonus: 32_767,
                redundancy_penalty: 0,
            }),
            source_ref: Some(ContextPackSourceRefV1 {
                source_id: "ifc:project-42".to_owned(),
                source_url: Some("https://example.test/project/42".to_owned()),
                document_id: Some("doc-42".to_owned()),
                page: Some(7),
                row: None,
                cell_range: Some("chunk-0003".to_owned()),
                json_path: None,
                confidence_q16: 61_000,
            }),
            provenance: Some(ContextPackProvenanceV1 {
                source_cell_id: 42,
                source_byte_start: 88,
                source_byte_end: 132,
                source_line_start: 4,
                source_line_end: 4,
                source_ref: Some(ContextPackSourceRefV1 {
                    source_id: "ifc:project-42".to_owned(),
                    source_url: Some("https://example.test/project/42".to_owned()),
                    document_id: Some("doc-42".to_owned()),
                    page: Some(7),
                    row: None,
                    cell_range: Some("chunk-0003".to_owned()),
                    json_path: None,
                    confidence_q16: 61_000,
                }),
            }),
            access_decision: Some(ContextPackAccessDecisionV1 {
                cell_id: 42,
                decision: "allowed".to_owned(),
                policy: "agent_view_readable_scope".to_owned(),
                reason:
                    "cell scope was present in AgentView.readable_scopes before ContextPack packing"
                        .to_owned(),
                scope: "project:investments".to_owned(),
                scope_id: 123,
                agent_id: Some(7),
                principal_id: Some("analyst-7".to_owned()),
                auth_role: Some("data".to_owned()),
            }),
        }],
        anomalies: vec![ContextPackAnomalyV1 {
            cell_id: Some(77),
            code: "token_overload".to_owned(),
            message: "candidate exceeded remaining token budget".to_owned(),
            why_excluded: Some("large_cell_policy=exclude".to_owned()),
        }],
    };

    assert!(pack.is_v1());
    assert!(!pack.is_over_budget());
    assert_eq!(pack.cell_ids().collect::<Vec<_>>(), vec![42]);
    assert_eq!(pack.citation_count(), 1);
    assert_eq!(pack.anomaly_count("token_overload"), 1);

    let encoded = serde_json::to_string(&pack).expect("context pack should serialize");
    let decoded: ContextPackV1 =
        serde_json::from_str(&encoded).expect("context pack should deserialize");

    assert_eq!(decoded, pack);
    assert_eq!(
        decoded.cells[0]
            .source_ref
            .as_ref()
            .and_then(|source| source.source_url.as_deref()),
        Some("https://example.test/project/42")
    );
    assert_eq!(
        decoded.cells[0]
            .explain
            .as_ref()
            .map(|explain| explain.score_components[0].name.as_str()),
        Some("source_trust")
    );
    assert_eq!(
        decoded.cells[0]
            .provenance
            .as_ref()
            .map(|provenance| provenance.source_line_start),
        Some(4)
    );
    assert_eq!(
        decoded.cells[0]
            .provenance
            .as_ref()
            .and_then(|provenance| provenance.source_ref.as_ref())
            .map(|source_ref| source_ref.source_id.as_str()),
        Some("ifc:project-42")
    );
    assert_eq!(
        decoded.cells[0]
            .access_decision
            .as_ref()
            .map(|decision| (decision.decision.as_str(), decision.auth_role.as_deref())),
        Some(("allowed", Some("data")))
    );
}

#[test]
fn context_pack_v1_deserializes_without_optional_provenance() {
    let decoded: ContextPackV1 = serde_json::from_str(
        r#"{
          "schema_version":"context_pack.v1",
          "token_budget_tokens":128,
          "estimated_tokens":45,
          "truncated":false,
          "citations_required":false,
          "answerability_q16":65535,
          "conflict_visibility_q16":0,
          "visible_conflict_count":0,
          "cells":[{"cell_id":1,"estimated_tokens":45,"citation":null,"payload_text":"body","explain":null,"source_ref":null}],
          "anomalies":[]
        }"#,
    )
    .expect("old context pack shape should remain readable");

    assert_eq!(decoded.cells[0].provenance, None);
    assert_eq!(decoded.cells[0].access_decision, None);
}

#[test]
fn context_pack_v1_helpers_detect_schema_budget_and_anomalies() {
    let pack = ContextPackV1 {
        schema_version: "future_context_pack.v2".to_owned(),
        token_budget_tokens: 4,
        estimated_tokens: 12,
        truncated: true,
        citations_required: false,
        answerability_q16: 0,
        conflict_visibility_q16: 0,
        visible_conflict_count: 0,
        cells: vec![ContextPackCellV1 {
            cell_id: 9,
            estimated_tokens: 12,
            citation: None,
            payload_text: "oversized body".to_owned(),
            explain: None,
            source_ref: None,
            provenance: None,
            access_decision: None,
        }],
        anomalies: vec![
            ContextPackAnomalyV1 {
                cell_id: Some(9),
                code: "token_overload".to_owned(),
                message: "over budget".to_owned(),
                why_excluded: None,
            },
            ContextPackAnomalyV1 {
                cell_id: None,
                code: "insufficient_context".to_owned(),
                message: "missing evidence".to_owned(),
                why_excluded: None,
            },
        ],
    };

    assert!(!pack.is_v1());
    assert!(pack.is_over_budget());
    assert_eq!(pack.citation_count(), 0);
    assert_eq!(pack.anomaly_count("token_overload"), 1);
    assert_eq!(pack.anomaly_count("insufficient_context"), 1);
    assert_eq!(pack.anomaly_count("missing_citation"), 0);
}

#[test]
fn context_pack_v1_grounding_helper_flags_unsupported_answer_spans() {
    let pack = ContextPackV1 {
        schema_version: ContextPackV1::SCHEMA_VERSION_V1.to_owned(),
        token_budget_tokens: 128,
        estimated_tokens: 32,
        truncated: false,
        citations_required: true,
        answerability_q16: 65_535,
        conflict_visibility_q16: 0,
        visible_conflict_count: 0,
        cells: vec![ContextPackCellV1 {
            cell_id: 1,
            estimated_tokens: 32,
            citation: Some("report-q1".to_owned()),
            payload_text: "Solar Plant budget is 1.2B KZT.".to_owned(),
            explain: None,
            source_ref: None,
            provenance: None,
            access_decision: None,
        }],
        anomalies: Vec::new(),
    };

    let report = pack.ground_answer_with_options(
        "Solar Plant budget is 1.2B KZT. The project TTL is 45 days.",
        AnswerGroundingOptionsV1 {
            reject_unsupported: true,
            ..AnswerGroundingOptionsV1::default()
        },
    );

    assert!(!report.answer_supported);
    assert!(report.rejected);
    assert_eq!(report.supported_span_count, 1);
    assert_eq!(report.unsupported_span_count, 1);
    assert_eq!(report.spans[0].citations, vec!["report-q1".to_owned()]);
    assert!(report.spans[1].missing_terms.contains(&"ttl".to_owned()));
}
