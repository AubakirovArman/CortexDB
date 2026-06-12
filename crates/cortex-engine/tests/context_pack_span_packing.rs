use cortex_core::CellId;
use cortex_engine::{
    ContextLargeCellPolicy, ContextPack, ContextPackExportFormat, ContextPackOptions, RetrievedCell,
};

#[test]
fn span_level_packing_beats_prefix_truncation_under_same_budget() {
    let budget = 128;
    let prefix = pack_with_options(ContextPackOptions {
        token_budget_tokens: budget,
        large_cell_policy: ContextLargeCellPolicy::Truncate,
        span_level_packing: false,
        ..ContextPackOptions::default()
    });
    let span = pack_with_options(ContextPackOptions {
        token_budget_tokens: budget,
        large_cell_policy: ContextLargeCellPolicy::Truncate,
        span_level_packing: true,
        span_context_lines: 1,
        ..ContextPackOptions::default()
    });

    assert!(prefix.estimated_tokens <= budget);
    assert!(span.estimated_tokens <= budget);
    assert_eq!(coverage(&prefix), 0);
    assert_eq!(coverage(&span), 3);
    assert!(payload(&span).contains("[context_pack_span=true"));
    assert!(!payload(&span).contains("intro filler intro filler intro filler"));
    let provenance = span.cells[0].provenance.as_ref().unwrap();
    assert_eq!(provenance.source_cell_id, CellId(9));
    assert_eq!(provenance.source_line_start, 2);
    assert_eq!(provenance.source_line_end, 2);
    assert!(provenance.source_byte_end > provenance.source_byte_start);
    assert_eq!(
        provenance
            .source_ref
            .as_ref()
            .map(|source_ref| source_ref.source_id.as_str()),
        Some("apollo-runbook")
    );
}

#[test]
fn span_level_packing_preserves_citation_metadata() {
    let pack = ContextPack::from_retrieved_with_options(
        vec![retrieved(9, &large_payload())],
        128,
        true,
        &ContextPackOptions {
            token_budget_tokens: 128,
            require_citations: true,
            large_cell_policy: ContextLargeCellPolicy::Truncate,
            span_level_packing: true,
            span_context_lines: 0,
            ..ContextPackOptions::default()
        },
        query(),
    );

    assert_eq!(pack.cells.len(), 1);
    assert_eq!(pack.cells[0].citation.as_deref(), Some("apollo-runbook"));
    assert!(payload(&pack).contains("source=apollo-runbook"));
    assert!(payload(&pack).contains("doc_id=apollo-doc"));
    assert!(payload(&pack).contains("Apollo migration owner is Maya"));
    assert!(pack.estimated_tokens <= pack.token_budget_tokens);

    let json = pack.export(ContextPackExportFormat::Json);
    assert!(json.contains(r#""provenance""#));
    assert!(json.contains(r#""source_line_start":2"#));
    let prompt = pack.export(ContextPackExportFormat::Prompt);
    assert!(prompt.contains("provenance=source_cell_id=9"));
    let markdown = pack.export(ContextPackExportFormat::Markdown);
    assert!(markdown.contains("- provenance: `source_cell_id=9"));
}

#[test]
fn span_packed_cells_export_structured_provenance() {
    let pack = pack_with_options(ContextPackOptions {
        token_budget_tokens: 128,
        large_cell_policy: ContextLargeCellPolicy::Truncate,
        span_level_packing: true,
        span_context_lines: 0,
        ..ContextPackOptions::default()
    });

    let provenance = pack.cells[0].provenance.as_ref().unwrap();
    assert_eq!(provenance.source_cell_id, CellId(9));
    assert_eq!(provenance.source_line_start, 2);
    assert_eq!(provenance.source_line_end, 2);

    let json = pack.export(ContextPackExportFormat::Json);
    assert!(json.contains(r#""source_cell_id":9"#));
    assert!(json.contains(r#""source_byte_start":"#));
    assert!(json.contains(r#""source_ref""#));

    let prompt = pack.export(ContextPackExportFormat::Prompt);
    assert!(prompt.contains("provenance=source_cell_id=9"));
    let markdown = pack.export(ContextPackExportFormat::Markdown);
    assert!(markdown.contains("- provenance: `source_cell_id=9"));
}

fn pack_with_options(options: ContextPackOptions) -> ContextPack {
    ContextPack::from_retrieved_with_options(
        vec![retrieved(9, &large_payload())],
        options.token_budget_tokens,
        false,
        &options,
        query(),
    )
}

fn coverage(pack: &ContextPack) -> usize {
    let text = payload(pack);
    ["Maya", "Friday", "Payments"]
        .iter()
        .filter(|term| text.contains(*term))
        .count()
}

fn payload(pack: &ContextPack) -> String {
    String::from_utf8_lossy(&pack.cells[0].payload).to_string()
}

fn large_payload() -> String {
    format!(
        "scope=project:investments\nstatus=ready\nsource=apollo-runbook\ndoc_id=apollo-doc\nchunk_id=apollo-chunk\ntitle=Project Apollo Migration\n\n{}\nApollo migration owner is Maya, deadline is Friday, and the impacted service is Payments.\n{}",
        "intro filler ".repeat(180),
        "appendix filler ".repeat(180)
    )
}

fn retrieved(id: u64, payload: &str) -> RetrievedCell {
    RetrievedCell::from_payload(CellId(id), payload.as_bytes().to_vec())
}

fn query() -> &'static str {
    r#"RETRIEVE CONTEXT FOR TASK "Apollo migration owner deadline Payments" IN BRAIN investment_projects;"#
}
