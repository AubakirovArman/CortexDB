use super::*;

#[test]
fn verify_request_fact_builder_outputs_stable_path_and_body() {
    let request = VerifyRequest::fact(
        "project:investments",
        "Solar Plant budget is 1.2B KZT",
        "investment_projects",
    )
    .expect("verify request should build")
    .markdown();

    assert_eq!(request.scope(), "project:investments");
    assert_eq!(request.format(), VerifyOutputFormat::Markdown);
    assert_eq!(
        request.path(),
        "/v1/verify?scope=project%3Ainvestments&format=markdown"
    );
    assert_eq!(
        request.statement(),
        "VERIFY FACT \"Solar Plant budget is 1.2B KZT\" IN BRAIN investment_projects;"
    );
}

#[test]
fn verify_request_formats_are_wire_stable() {
    let json = VerifyRequest::new("project:investments", "VERIFY FACT \"x\" IN BRAIN b;");
    let audit = json.clone().audit();

    assert_eq!(json.format(), VerifyOutputFormat::Json);
    assert_eq!(
        json.path(),
        "/v1/verify?scope=project%3Ainvestments&format=json"
    );
    assert_eq!(
        audit.path(),
        "/v1/verify?scope=project%3Ainvestments&format=audit"
    );
}

#[test]
fn verify_result_maps_current_and_legacy_wire_statuses() {
    assert_eq!(
        VerifyResult::from_wire("supported"),
        VerifyResult::Supported
    );
    assert_eq!(
        VerifyResult::from_wire("insufficient"),
        VerifyResult::Insufficient
    );
    assert_eq!(
        VerifyResult::from_wire("contradicted"),
        VerifyResult::Contradicted
    );
    assert_eq!(
        VerifyResult::from_wire("mixed"),
        VerifyResult::MixedEvidence
    );
    assert_eq!(
        VerifyResult::from_wire("mixed_evidence"),
        VerifyResult::MixedEvidence
    );
    assert_eq!(
        VerifyResult::from_wire("future_status"),
        VerifyResult::Unknown("future_status".to_owned())
    );
}

#[test]
fn verification_report_helpers_surface_result_and_conflicts() {
    let value = serde_json::json!({
        "fact": "Solar Plant budget is 1.2B KZT",
        "status": "mixed",
        "verdict": "mixed_evidence",
        "evidence": [],
        "contradicting_evidence": [{
            "cell_id": 7,
            "matched_terms": 3,
            "source_trust_q16": 60000,
            "source_trust_category": "official",
            "citation": "ifc:project-7",
            "payload_text": "scope=project:investments\nvalue=1.4B KZT"
        }],
        "guards": [],
        "supporting": [],
        "contradicting": [],
        "numeric_conflicts": [{
            "metric": "budget",
            "left": "1.2B KZT",
            "right": "1.4B KZT"
        }]
    });

    let report: VerificationReportResponse =
        serde_json::from_value(value).expect("verification report should decode");

    assert_eq!(report.result(), VerifyResult::MixedEvidence);
    assert!(report.result().is_actionable_conflict());
    assert!(report.has_conflicts());

    let conflicts = report.conflicts();
    assert_eq!(conflicts.len(), 2);
    assert!(matches!(
        &conflicts[0],
        VerifyConflict::ContradictingEvidence(conflict)
            if conflict.cell_id == 7 && conflict.source_trust_category == "official"
    ));
    assert!(matches!(
        &conflicts[1],
        VerifyConflict::Numeric(conflict)
            if conflict.metric == "budget" && conflict.left == "1.2B KZT"
    ));
}
