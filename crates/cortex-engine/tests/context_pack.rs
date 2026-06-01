use std::collections::BTreeSet;

use cortex_aql::{AgentId, AgentView, BrainId, MemoryType, RetrievalMode, Q16_ZERO};
use cortex_core::CellId;
use cortex_engine::feedback::ContextFeedback;
use cortex_engine::{
    scope_id, ContextPack, ContextPackAnomalyCode, ContextPackExportFormat, ContextPackOptions,
    Database, RetrievedCell, SourceTrustCategory, DEFAULT_CITATION_OVERHEAD_TOKENS,
};

#[test]
fn context_pack_from_aql_respects_budget() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_cell(
        CellId(1),
        b"scope=project:investments\nstatus=ready\nsource=doc-a\nsmall".to_vec(),
    )
    .unwrap();
    db.put_cell(
        CellId(2),
        b"scope=project:investments\nstatus=ready\nsource=doc-b\nthis payload is deliberately much larger than the first selected cell".to_vec(),
    )
    .unwrap();

    let pack = db
        .context_pack_from_aql(
            query(),
            &view(false),
            ContextPackOptions {
                token_budget_tokens: 16,
                require_citations: false,
                ..ContextPackOptions::default()
            },
        )
        .unwrap();

    assert_eq!(pack.cells.len(), 1);
    assert_eq!(pack.cells[0].cell_id, CellId(1));
    assert!(pack.truncated);
    assert!(pack.estimated_tokens <= pack.token_budget_tokens);
    assert_eq!(
        pack.anomalies[0].code,
        ContextPackAnomalyCode::TokenOverload
    );
    assert!(pack.anomalies[0]
        .why_excluded
        .as_deref()
        .unwrap_or_default()
        .contains("token_budget_tokens"));
}

#[test]
fn context_pack_reports_missing_citations_when_required() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_cell(
        CellId(1),
        b"scope=project:investments\nstatus=ready\nalpha budget".to_vec(),
    )
    .unwrap();

    let pack = db
        .context_pack_from_aql(query(), &view(true), ContextPackOptions::default())
        .unwrap();

    assert!(pack.citations_required);
    assert_eq!(pack.anomalies.len(), 1);
    assert_eq!(
        pack.anomalies[0].code,
        ContextPackAnomalyCode::MissingCitation
    );
    assert_eq!(pack.anomalies[0].cell_id, Some(CellId(1)));
    assert_eq!(pack.anomalies[0].why_excluded, None);
}

#[test]
fn context_pack_uses_source_line_as_citation() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_cell(
        CellId(1),
        b"scope=project:investments\nstatus=ready\nsource=annual-report\nalpha budget".to_vec(),
    )
    .unwrap();

    let pack = db
        .context_pack_from_aql(query(), &view(true), ContextPackOptions::default())
        .unwrap();

    assert!(pack.anomalies.is_empty());
    assert_eq!(pack.cells[0].citation.as_deref(), Some("annual-report"));
}

#[test]
fn context_pack_accounts_for_required_citation_overhead() {
    let payload = "scope=project:investments\nstatus=ready\nsource=doc-a\nalpha budget";
    let pack = ContextPack::from_retrieved_with_options(
        vec![retrieved(1, payload)],
        1_000,
        true,
        &ContextPackOptions::default(),
        "",
    );

    assert_eq!(
        pack.cells[0].estimated_tokens,
        cortex_engine::estimate_tokens(payload.as_bytes()) + DEFAULT_CITATION_OVERHEAD_TOKENS
    );
    assert_eq!(pack.estimated_tokens, pack.cells[0].estimated_tokens);
}

#[test]
fn context_pack_skips_oversized_middle_candidate_and_keeps_later_fit() {
    let huge = "x ".repeat(200);
    let pack = ContextPack::from_retrieved_with_options(
        vec![
            retrieved(1, "small first"),
            retrieved(2, &huge),
            retrieved(3, "tiny"),
        ],
        12,
        false,
        &ContextPackOptions::default(),
        "",
    );

    assert_eq!(
        pack.cells
            .iter()
            .map(|cell| cell.cell_id)
            .collect::<Vec<_>>(),
        vec![CellId(1), CellId(3)]
    );
    assert!(pack.truncated);
    assert_eq!(
        pack.anomalies[0].code,
        ContextPackAnomalyCode::TokenOverload
    );
}

#[test]
fn context_pack_applies_redundancy_before_budget_overload() {
    let huge_duplicate = "alpha budget project ".repeat(100);
    let pack = ContextPack::from_retrieved_with_options(
        vec![
            retrieved(1, "alpha budget project"),
            retrieved(2, &huge_duplicate),
            retrieved(3, "gamma schedule"),
        ],
        16,
        false,
        &ContextPackOptions {
            reduce_redundancy: true,
            redundancy_threshold_q16: 10,
            ..ContextPackOptions::default()
        },
        "",
    );

    assert_eq!(
        pack.cells
            .iter()
            .map(|cell| cell.cell_id)
            .collect::<Vec<_>>(),
        vec![CellId(1), CellId(3)]
    );
    assert!(!pack.truncated);
    assert_eq!(
        pack.anomalies[0].code,
        ContextPackAnomalyCode::RedundantCell
    );
}

#[test]
fn context_pack_exports_stable_prompt_and_markdown() {
    let cells = vec![retrieved(
        7,
        "scope=project:investments\nstatus=ready\nsource=doc-a\nsource_id=doc-a\ndocument_id=doc-1\npage=3\n\nSolar budget evidence.",
    )];
    let pack = ContextPack::from_retrieved_with_options(
        cells,
        1_000,
        true,
        &ContextPackOptions::default(),
        "RETRIEVE CONTEXT FOR TASK \"budget\" IN BRAIN investment_projects;",
    );

    let prompt = pack.export(ContextPackExportFormat::Prompt);
    assert!(prompt.contains("CortexDB ContextPack v1"));
    assert!(prompt.contains("Use only the context cells below."));
    assert!(prompt.contains("[1] cell_id=7"));
    assert!(prompt.contains("source_ref=source_id=doc-a;document_id=doc-1;page=3"));
    assert!(prompt.contains("Solar budget evidence."));

    let markdown = pack.export(ContextPackExportFormat::Markdown);
    assert!(markdown.contains("# CortexDB ContextPack"));
    assert!(markdown.contains("### Cell 1"));
    assert!(markdown.contains("- cell_id: `7`"));
    assert!(markdown.contains("```text"));
    assert!(markdown.contains("Solar budget evidence."));
}

#[test]
fn context_pack_markdown_export_preserves_code_fences() {
    let cells = vec![retrieved(
        8,
        "scope=project:investments\nstatus=ready\nsource=doc-a\n\npayload with ``` fenced text",
    )];
    let pack = ContextPack::from_retrieved(cells, 1_000, false);

    let markdown = pack.export(ContextPackExportFormat::Markdown);
    assert!(markdown.contains("````text"));
    assert!(markdown.contains("payload with ``` fenced text"));
}

#[test]
fn context_pack_orders_cells_by_feedback_score() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_cell(
        CellId(1),
        b"scope=project:investments\nstatus=ready\nsource=doc-a\nalpha budget".to_vec(),
    )
    .unwrap();
    db.put_cell(
        CellId(2),
        b"scope=project:investments\nstatus=ready\nsource=doc-b\nbeta budget".to_vec(),
    )
    .unwrap();
    db.record_context_feedback(
        AgentId(1),
        ContextFeedback {
            source_cell_id: CellId(2),
            useful: true,
            note: None,
        },
    )
    .unwrap();

    let pack = db
        .context_pack_from_aql(query(), &view(false), ContextPackOptions::default())
        .unwrap();
    assert_eq!(pack.cells[0].cell_id, CellId(2));
}

#[test]
fn context_pack_can_reduce_sparse_redundancy() {
    let cells = vec![
        retrieved(1, "alpha budget project"),
        retrieved(2, "alpha budget project duplicate"),
        retrieved(3, "gamma schedule"),
    ];
    let pack = ContextPack::from_retrieved_with_options(
        cells,
        1_000,
        false,
        &ContextPackOptions {
            reduce_redundancy: true,
            redundancy_threshold_q16: 32_768,
            ..ContextPackOptions::default()
        },
        "",
    );

    assert_eq!(
        pack.cells
            .iter()
            .map(|cell| cell.cell_id)
            .collect::<Vec<_>>(),
        vec![CellId(1), CellId(3)]
    );
    assert_eq!(
        pack.anomalies[0].code,
        ContextPackAnomalyCode::RedundantCell
    );
    assert_eq!(pack.anomalies[0].cell_id, Some(CellId(2)));
    assert!(pack.anomalies[0]
        .why_excluded
        .as_deref()
        .unwrap_or_default()
        .contains("reduce_redundancy"));
}

#[test]
fn context_pack_can_reduce_dense_vector_redundancy() {
    let cells = vec![
        retrieved(1, "scope=project\nvector=1, 2, 3\nfirst cell"),
        retrieved(2, "scope=project\nvector=1, 2, 4\nsecond cell"),
        retrieved(3, "scope=project\nvector=-1, -2, -3\nthird cell"),
    ];
    let pack = ContextPack::from_retrieved_with_options(
        cells,
        1_000,
        false,
        &ContextPackOptions {
            reduce_redundancy: true,
            redundancy_threshold_q16: 32_768,
            ..ContextPackOptions::default()
        },
        "",
    );

    assert_eq!(
        pack.cells
            .iter()
            .map(|cell| cell.cell_id)
            .collect::<Vec<_>>(),
        vec![CellId(1), CellId(3)]
    );
    assert_eq!(
        pack.anomalies[0].code,
        ContextPackAnomalyCode::RedundantCell
    );
    assert_eq!(pack.anomalies[0].cell_id, Some(CellId(2)));
    assert!(pack.anomalies[0]
        .why_excluded
        .as_deref()
        .unwrap_or_default()
        .contains("reduce_redundancy"));
}

#[test]
fn context_pack_keeps_redundant_cells_by_default() {
    let cells = vec![
        retrieved(1, "alpha budget project"),
        retrieved(2, "alpha budget project duplicate"),
    ];
    let pack = ContextPack::from_retrieved(cells, 1_000, false);

    assert_eq!(pack.cells.len(), 2);
    assert!(pack.anomalies.is_empty());
}

fn query() -> &'static str {
    r#"RETRIEVE CONTEXT FOR TASK "budget" IN BRAIN investment_projects
WHERE space = project:investments AND status = "ready" LIMIT 10 CANDIDATES;"#
}

fn retrieved(cell_id: u64, payload: &str) -> RetrievedCell {
    RetrievedCell {
        cell_id: CellId(cell_id),
        payload: payload.as_bytes().to_vec(),
    }
}

fn view(require_citations: bool) -> AgentView {
    AgentView {
        agent_id: AgentId(1),
        label: None,
        readable_brains: BTreeSet::from([BrainId(1)]),
        readable_scopes: BTreeSet::from([scope_id("project:investments")]),
        writable_scopes: BTreeSet::new(),
        allowed_modes: BTreeSet::from([RetrievalMode::Balanced]),
        allowed_memory_types: BTreeSet::from([MemoryType::Decision]),
        max_context_budget_tokens: 1_000,
        default_context_budget_tokens: 400,
        max_candidate_limit: 100,
        default_candidate_limit: 20,
        min_required_confidence_q16: Q16_ZERO,
        max_ttl_seconds: Some(3_600),
        allow_remember: false,
        allow_verify_fact: false,
        allow_audit_mode: false,
        require_citations_by_default: require_citations,
        private_scope: None,
    }
}

#[test]
fn test_deterministic_cosine_similarity() {
    let cells = vec![
        retrieved(1, "scope=project\nvector=100, 200, 300\ncell 1"),
        retrieved(2, "scope=project\nvector=100, 200, 300\ncell 2"), // Identical vectors!
    ];
    let pack = ContextPack::from_retrieved_with_options(
        cells,
        1_000,
        false,
        &ContextPackOptions {
            reduce_redundancy: true,
            redundancy_threshold_q16: 65535, // Exact match
            ..ContextPackOptions::default()
        },
        "",
    );
    // Pruned cell 2 because the vectors are exactly identical (cosine similarity = 65535 >= threshold)
    assert_eq!(pack.cells.len(), 1);
    assert_eq!(pack.anomalies.len(), 1);
}

#[test]
fn test_numeric_guard_coexistence() {
    let cells = vec![
        retrieved(1, "scope=project:investments\nstatus=ready\nproject=Solar\nmetric=budget\nvalue=1200000000\ncurrency=KZT\n\nSolar Plant budget is 1.2B"),
        retrieved(2, "scope=project:investments\nstatus=ready\nproject=Solar\nmetric=budget\nvalue=1400000000\ncurrency=KZT\n\nSolar Plant budget is 1.4B"), // Conflicting values!
    ];
    let pack = ContextPack::from_retrieved_with_options(
        cells,
        2_000,
        false,
        &ContextPackOptions {
            reduce_redundancy: true,
            redundancy_threshold_q16: 10, // Very low threshold, but should not prune!
            ..ContextPackOptions::default()
        },
        "",
    );
    // Both are kept together because they represent different values for same project+metric (Numeric Guard!)
    assert_eq!(pack.cells.len(), 2);
}

#[test]
fn test_context_pack_scoring_and_explain() {
    let cells = vec![
        retrieved(1, "scope=project:investments\nstatus=ready\nsource=report_q1\nproject=Solar\nmetric=budget\nvalue=12\ncurrency=KZT\n\nSolar Plant budget is 1.2B"),
    ];
    let pack = ContextPack::from_retrieved_with_options(
        cells,
        1_000,
        false,
        &ContextPackOptions::default(),
        "RETRIEVE CONTEXT FOR TASK \"budget\" IN BRAIN investment_projects WHERE space = project:investments LIMIT 10 CANDIDATES;",
    );
    let cell = &pack.cells[0];
    let exp = cell.explain.as_ref().unwrap();
    assert_eq!(exp.matched_terms, vec!["budget"]);
    assert_eq!(exp.base_bm25, 10_000);
    assert!(exp.score > 0);
    assert!(!exp.why_selected.is_empty());
    assert_eq!(exp.score_components.len(), 3);
    let component_names = exp
        .score_components
        .iter()
        .map(|component| component.name.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        component_names,
        vec!["base_bm25", "source_trust_bonus", "redundancy_penalty"]
    );
    assert_eq!(exp.source_trust_q16, 32_768);
    assert_eq!(exp.source_trust_category, SourceTrustCategory::Unknown);
    assert!(exp
        .score_components
        .iter()
        .any(|component| component.name == "redundancy_penalty" && component.contribution <= 0));
}

#[test]
fn context_pack_explain_reports_source_trust_category() {
    let cells = vec![retrieved(
        1,
        "scope=project:investments\nstatus=ready\nsource_trust_q16=60000\n\nalpha budget",
    )];
    let pack = ContextPack::from_retrieved_with_options(
        cells,
        1_000,
        false,
        &ContextPackOptions::default(),
        "RETRIEVE CONTEXT FOR TASK \"budget\" IN BRAIN investment_projects;",
    );
    let explain = pack.cells[0].explain.as_ref().unwrap();

    assert_eq!(explain.source_trust_q16, 60_000);
    assert_eq!(explain.source_trust_category, SourceTrustCategory::Official);
    assert!(explain
        .score_components
        .iter()
        .any(|component| component.name == "source_trust_bonus"
            && component.reason.contains("official provenance trust")));
}
