use crate::{
    ContextPackAnomalyV1, ContextPackCellV1, ContextPackExplainV1, ContextPackSourceRefV1,
    ContextPackV1, ScoreComponentResponse,
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
                redundancy_penalty: 0,
            }),
            source_ref: Some(ContextPackSourceRefV1 {
                source_id: "ifc:project-42".to_owned(),
                source_url: Some("https://example.test/project/42".to_owned()),
                document_id: Some("doc-42".to_owned()),
                page: Some(7),
                cell_range: Some("chunk-0003".to_owned()),
                json_path: None,
                confidence_q16: 61_000,
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
