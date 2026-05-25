use std::collections::BTreeSet;

use cortex_aql::{AgentId, AgentView, BrainId, MemoryType, RetrievalMode, Q16_ZERO};
use cortex_core::CellId;
use cortex_engine::{scope_id, ContextPack, ContextPackOptions, Database, RetrievedCell};

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
    assert_eq!(pack.anomalies[0].code, "missing_citation");
    assert_eq!(pack.anomalies[0].cell_id, Some(CellId(1)));
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
    );

    assert_eq!(
        pack.cells
            .iter()
            .map(|cell| cell.cell_id)
            .collect::<Vec<_>>(),
        vec![CellId(1), CellId(3)]
    );
    assert_eq!(pack.anomalies[0].code, "redundant_cell");
    assert_eq!(pack.anomalies[0].cell_id, Some(CellId(2)));
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
