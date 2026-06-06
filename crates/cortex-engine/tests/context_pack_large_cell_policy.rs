use cortex_core::CellId;
use cortex_engine::{
    ContextLargeCellPolicy, ContextPack, ContextPackAnomalyCode, ContextPackOptions, RetrievedCell,
};

#[test]
fn truncate_policy_keeps_prefix_within_budget() {
    let pack = pack_with_policy(ContextLargeCellPolicy::Truncate, 80);

    assert_eq!(pack.cells.len(), 1);
    assert!(pack.truncated);
    assert!(pack.estimated_tokens <= pack.token_budget_tokens);
    let payload = payload_text(&pack);
    assert!(payload.contains("scope=project:investments"));
    assert!(payload.contains("[context_pack_truncated=true]"));
    assert!(!payload.contains("NEVER_INCLUDE_TAIL"));
    assert_policy_anomaly(&pack, "large_cell_policy=truncate included");
}

#[test]
fn exclude_policy_drops_oversized_cell() {
    let pack = pack_with_policy(ContextLargeCellPolicy::Exclude, 80);

    assert!(pack.cells.is_empty());
    assert!(pack.truncated);
    assert_policy_anomaly(&pack, "large_cell_policy=exclude excluded");
}

#[test]
fn summarize_placeholder_policy_keeps_deterministic_reference() {
    let pack = pack_with_policy(ContextLargeCellPolicy::SummarizePlaceholder, 100);

    assert_eq!(pack.cells.len(), 1);
    assert!(pack.estimated_tokens <= pack.token_budget_tokens);
    let payload = payload_text(&pack);
    assert!(payload.contains("summary_placeholder=true"));
    assert!(payload.contains("original_cell_id=7"));
    assert!(payload.contains("title=Large Budget Report"));
    assert!(!payload.contains("NEVER_INCLUDE_TAIL"));
    assert_policy_anomaly(&pack, "large_cell_policy=summarize_placeholder included");
}

#[test]
fn source_only_reference_policy_keeps_provenance_without_body() {
    let pack = pack_with_policy(ContextLargeCellPolicy::SourceOnlyReference, 100);

    assert_eq!(pack.cells.len(), 1);
    assert_eq!(pack.cells[0].citation.as_deref(), Some("doc-a"));
    assert!(pack.estimated_tokens <= pack.token_budget_tokens);
    let payload = payload_text(&pack);
    assert!(payload.contains("source_only_reference=true"));
    assert!(payload.contains("doc_id=doc-1"));
    assert!(payload.contains("chunk_id=chunk-1"));
    assert!(!payload.contains("NEVER_INCLUDE_TAIL"));
    assert_policy_anomaly(&pack, "large_cell_policy=source_only_reference included");
}

fn pack_with_policy(policy: ContextLargeCellPolicy, budget: u32) -> ContextPack {
    ContextPack::from_retrieved_with_options(
        vec![retrieved(7, &large_payload())],
        budget,
        true,
        &ContextPackOptions {
            token_budget_tokens: budget,
            require_citations: true,
            large_cell_policy: policy,
            ..ContextPackOptions::default()
        },
        "RETRIEVE CONTEXT FOR TASK \"budget evidence\" IN BRAIN investment_projects;",
    )
}

fn large_payload() -> String {
    let repeated = "budget evidence ".repeat(240);
    format!(
        "scope=project:investments\nstatus=ready\nsource=doc-a\ndoc_id=doc-1\nchunk_id=chunk-1\ntitle=Large Budget Report\n\n{repeated}NEVER_INCLUDE_TAIL"
    )
}

fn retrieved(id: u64, payload: &str) -> RetrievedCell {
    RetrievedCell {
        cell_id: CellId(id),
        payload: payload.as_bytes().to_vec(),
    }
}

fn payload_text(pack: &ContextPack) -> String {
    String::from_utf8_lossy(&pack.cells[0].payload).to_string()
}

fn assert_policy_anomaly(pack: &ContextPack, expected: &str) {
    assert!(pack.anomalies.iter().any(|anomaly| anomaly.code
        == ContextPackAnomalyCode::TokenOverload
        && anomaly
            .why_excluded
            .as_deref()
            .unwrap_or_default()
            .contains(expected)));
}
