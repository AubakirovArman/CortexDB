use cortex_core::CellId;
use cortex_engine::{ContextPack, ContextPackExportFormat, ContextPackOptions, RetrievedCell};
use serde_json::Value;

fn retrieved(cell_id: u64, payload: &str) -> RetrievedCell {
    RetrievedCell {
        cell_id: CellId(cell_id),
        payload: payload.as_bytes().to_vec(),
    }
}

fn export_pack() -> ContextPack {
    let cells = vec![retrieved(
        7,
        "scope=project:investments\nstatus=ready\nsource=doc-a\nsource_id=doc-a\ndocument_id=doc-1\npage=3\nsource_trust_q16=60000\n\nSolar budget evidence.",
    )];
    ContextPack::from_retrieved_with_options(
        cells,
        1_000,
        true,
        &ContextPackOptions::default(),
        "RETRIEVE CONTEXT FOR TASK \"budget\" IN BRAIN investment_projects;",
    )
}

#[test]
fn context_pack_prompt_export_includes_citation_and_conflict_instructions() {
    let prompt = export_pack().export(ContextPackExportFormat::Prompt);

    assert!(prompt.contains("Use only the context cells below."));
    assert!(prompt.contains("Preserve citations when answering."));
    assert!(prompt.contains("Cite citation= or source_ref= values for factual claims."));
    assert!(prompt.contains("If the supplied context is insufficient or conflicting, say so."));
    assert!(prompt.contains("Do not resolve conflicting evidence silently"));
    assert!(prompt.contains("[1] cell_id=7"));
    assert!(prompt.contains("source_ref=source_id=doc-a;document_id=doc-1;page=3"));
    assert!(prompt.contains("Solar budget evidence."));
}

#[test]
fn context_pack_markdown_export_is_stable_and_cited() {
    let markdown = export_pack().export(ContextPackExportFormat::Markdown);

    assert!(markdown.contains("# CortexDB ContextPack"));
    assert!(markdown.contains("### Cell 1"));
    assert!(markdown.contains("- cell_id: `7`"));
    assert!(markdown.contains("- citation: `doc-a`"));
    assert!(markdown.contains("- source_ref: `source_id=doc-a;document_id=doc-1;page=3"));
    assert!(markdown.contains("- source_trust: `official` (`60000`)"));
    assert!(markdown.contains("```text"));
    assert!(markdown.contains("Solar budget evidence."));
}

#[test]
fn context_pack_json_export_has_public_schema_fields() {
    let json = export_pack().export(ContextPackExportFormat::Json);
    let value: Value = serde_json::from_str(&json).unwrap();

    assert_eq!(value["schema_version"], "context_pack.v1");
    assert_eq!(value["token_budget_tokens"], 1_000);
    assert_eq!(value["citations_required"], true);
    assert_eq!(value["cells"][0]["cell_id"], 7);
    assert_eq!(value["cells"][0]["citation"], "doc-a");
    assert_eq!(value["cells"][0]["source_ref"]["source_id"], "doc-a");
    assert_eq!(
        value["cells"][0]["explain"]["source_trust_category"],
        "official"
    );
    assert!(value["cells"][0]["explain"]["why_selected"]
        .as_str()
        .unwrap()
        .contains("high provenance source trust"));
    assert!(value["cells"][0]["explain"]["score_components"]
        .as_array()
        .unwrap()
        .iter()
        .any(|component| component["name"] == "source_trust_bonus"));
}
