use std::collections::{BTreeMap, BTreeSet};

use cortex_aql::{AgentId, AgentView, BrainId, MemoryType, RetrievalMode};
use cortex_core::{CellDescriptor, CellId, KnowledgeCellType};
use cortex_engine::{
    scope_id, ContextAccessDecisionOutcome, ContextPack, ContextPackExportFormat,
    ContextPackOptions, RetrievedCell,
};
use serde_json::Value;

fn retrieved(cell_id: u64, payload: &str) -> RetrievedCell {
    RetrievedCell::from_payload(CellId(cell_id), payload.as_bytes().to_vec())
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
    assert!(prompt.contains("Answerability: answerability_q16=65535"));
    assert!(
        prompt.contains("Conflict visibility: conflict_visibility_q16=0 visible_conflict_count=0")
    );
    assert!(prompt.contains("[1] cell_id=7"));
    assert!(prompt.contains("source_ref=source_id=doc-a;document_id=doc-1;page=3"));
    assert!(prompt.contains("Solar budget evidence."));
}

#[test]
fn context_pack_markdown_export_is_stable_and_cited() {
    let markdown = export_pack().export(ContextPackExportFormat::Markdown);

    assert!(markdown.contains("# CortexDB ContextPack"));
    assert!(markdown.contains("- answerability_q16: `65535`"));
    assert!(markdown.contains("- conflict_visibility_q16: `0`"));
    assert!(markdown.contains("- visible_conflict_count: `0`"));
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
    assert_eq!(value["answerability_q16"], 65535);
    assert_eq!(value["conflict_visibility_q16"], 0);
    assert_eq!(value["visible_conflict_count"], 0);
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

#[test]
fn context_pack_export_uses_descriptor_metadata_over_payload_headers() {
    let descriptor = CellDescriptor {
        scope: "project:secure".to_owned(),
        status: "ready".to_owned(),
        cell_type: KnowledgeCellType::Fact,
        source: Some("descriptor-source".to_owned()),
        citation: Some("descriptor-citation".to_owned()),
        source_trust_q16: Some(60_000),
        created_unix_seconds: Some(200),
        ..CellDescriptor::default()
    };
    let cell = RetrievedCell {
        cell_id: CellId(42),
        payload: b"scope=project:payload\nstatus=ready\nsource=payload-source\nsource_id=payload-source\ndocument_id=payload-doc\nsource_trust_q16=1\ncreated_unix_seconds=10\n\nsecure evidence"
            .to_vec(),
        descriptor,
    };
    let view = AgentView {
        agent_id: AgentId(7),
        label: None,
        readable_brains: BTreeSet::from([BrainId(1)]),
        readable_scopes: BTreeSet::from([scope_id("project:secure")]),
        writable_scopes: BTreeSet::new(),
        allowed_modes: BTreeSet::from([RetrievalMode::Balanced]),
        allowed_memory_types: BTreeSet::from([MemoryType::Decision]),
        max_context_budget_tokens: 1_000,
        default_context_budget_tokens: 400,
        max_candidate_limit: 100,
        default_candidate_limit: 20,
        min_required_confidence_q16: 0,
        max_ttl_seconds: Some(3_600),
        allow_remember: false,
        allow_verify_fact: false,
        allow_audit_mode: false,
        require_citations_by_default: true,
        private_scope: None,
    };

    let pack = ContextPack::from_retrieved_with_feedback_options_and_view(
        vec![cell],
        1_000,
        true,
        &ContextPackOptions::default(),
        "secure evidence",
        &BTreeMap::new(),
        Some(&view),
    );

    assert_eq!(
        pack.cells[0].citation.as_deref(),
        Some("descriptor-citation")
    );
    assert_eq!(
        pack.cells[0]
            .metadata
            .source_ref
            .as_ref()
            .map(|source_ref| source_ref.source_id.as_str()),
        Some("descriptor-source")
    );
    let decision = pack.cells[0].access_decision.as_ref().unwrap();
    assert_eq!(decision.decision, ContextAccessDecisionOutcome::Allowed);
    assert_eq!(decision.scope, "project:secure");

    let json: Value = serde_json::from_str(&pack.export(ContextPackExportFormat::Json)).unwrap();
    assert_eq!(json["cells"][0]["citation"], "descriptor-citation");
    assert_eq!(
        json["cells"][0]["source_ref"]["source_id"],
        "descriptor-source"
    );
    assert_eq!(
        json["cells"][0]["access_decision"]["scope"],
        "project:secure"
    );

    let prompt = pack.export(ContextPackExportFormat::Prompt);
    assert!(prompt.contains("citation=descriptor-citation"));
    assert!(prompt.contains("source_ref=source_id=descriptor-source"));
    assert!(!prompt.contains("citation=payload-source"));
    assert!(!prompt.contains("source_ref=source_id=payload-source"));
}
