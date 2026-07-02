use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use cortex_core::CellId;
use cortex_engine::canonical::canonical_context_pack_bytes;
use cortex_engine::{
    AnswerGroundingOptions, ContextPack, ContextPackAnomaly, ContextPackAnomalyCode,
    ContextPackExportFormat, ContextPackOptions, RetrievedCell,
};
use serde_json::{json, Value};

#[test]
fn grounding_and_retrieval_incomplete_are_exported_and_hashed_pack_outputs() {
    let base = ContextPack::from_retrieved_with_feedback_options_view_and_anomalies(
        vec![retrieved(
            31,
            "scope=project:investments\nstatus=ready\ncitation=doc://solar#p1\n\nSolar budget mitigation is approved.",
        )],
        1_000,
        true,
        &ContextPackOptions::default(),
        "RETRIEVE CONTEXT FOR TASK \"solar budget mitigation\" IN BRAIN investment_projects;",
        &BTreeMap::new(),
        None,
        vec![ContextPackAnomaly {
            cell_id: None,
            code: ContextPackAnomalyCode::RetrievalIncomplete,
            message: "ANN visit budget was exhausted before traversal completed".to_owned(),
            why_excluded: Some(
                "reported because ANN search returned a visit_budget_exceeded fallback".to_owned(),
            ),
        }],
    );

    let ungrounded_bytes = canonical_context_pack_bytes(&base);
    let grounded = base.clone().with_grounded_answer(
        "Solar budget mitigation is approved.",
        AnswerGroundingOptions {
            require_citations: true,
            ..AnswerGroundingOptions::default()
        },
    );
    let unsupported = base.with_grounded_answer(
        "Lunar procurement timeline is blocked.",
        AnswerGroundingOptions {
            require_citations: true,
            ..AnswerGroundingOptions::default()
        },
    );

    let grounded_bytes = canonical_context_pack_bytes(&grounded);
    let unsupported_bytes = canonical_context_pack_bytes(&unsupported);
    let grounded_text = String::from_utf8(grounded_bytes.clone()).expect("canonical utf8");
    let json_export = grounded.export(ContextPackExportFormat::Json);

    assert_ne!(
        ungrounded_bytes, grounded_bytes,
        "attaching grounding_report must change canonical ContextPack bytes",
    );
    assert_ne!(
        grounded_bytes, unsupported_bytes,
        "different grounding results must produce different canonical ContextPack bytes",
    );
    assert!(grounded_text.contains(r#""grounding_report":{"answer_supported":true"#));
    assert!(grounded_text.contains(r#""code":"retrieval_incomplete""#));
    assert!(json_export.contains(r#""grounding_report":{"answer_supported":true"#));
    assert!(json_export.contains(r#""code":"retrieval_incomplete""#));
    assert!(
        grounded
            .grounding_report
            .as_ref()
            .expect("grounding report is attached")
            .answer_supported
    );

    write_report(json!({
        "schema_version": "cortexdb.pack_completeness_signal.report.v1",
        "status": "passed",
        "ann_budget_disclosure_gate": "ann-budget-disclosure-check",
        "retrieval_incomplete_hashed": grounded_text.contains(r#""code":"retrieval_incomplete""#),
        "grounding_report_hashed": grounded_text.contains(r#""grounding_report""#),
        "grounding_changes_canonical_bytes": ungrounded_bytes != grounded_bytes,
        "grounding_result_changes_canonical_bytes": grounded_bytes != unsupported_bytes,
    }));
}

fn retrieved(cell_id: u64, payload: &str) -> RetrievedCell {
    RetrievedCell::from_payload(CellId(cell_id), payload.as_bytes().to_vec())
}

fn write_report(report: Value) {
    let Some(path) = std::env::var_os("CORTEX_PACK_COMPLETENESS_SIGNAL_REPORT") else {
        return;
    };
    let path = PathBuf::from(path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create pack-completeness report parent");
    }
    fs::write(
        path,
        serde_json::to_string_pretty(&report).expect("serialize pack-completeness report") + "\n",
    )
    .expect("write pack-completeness report");
}
