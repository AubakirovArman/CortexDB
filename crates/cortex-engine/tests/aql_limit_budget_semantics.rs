use std::collections::BTreeSet;

use cortex_aql::{AgentId, AgentView, BrainId, MemoryType, RetrievalMode, Q16_ZERO};
use cortex_core::CellId;
use cortex_engine::{scope_id, ContextPackOptions, Database};

#[test]
fn limit_candidates_bounds_retrieve_and_context_pack_cells() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    seed_ready_cells(&mut db, 4);
    let query = r#"RETRIEVE CONTEXT FOR TASK "budget" IN BRAIN investment_projects
WHERE space = project:investments AND status = "ready"
LIMIT 2 CANDIDATES;"#;

    let view = view(100, 20, 1_000, 400);
    let retrieved = db.retrieve_aql(query, &view).unwrap();
    let pack = db
        .context_pack_from_aql(query, &view, ContextPackOptions::default())
        .unwrap();

    assert_eq!(cell_ids(&retrieved), vec![CellId(1), CellId(2)]);
    assert_eq!(pack_cell_ids(&pack), vec![CellId(1), CellId(2)]);
}

#[test]
fn missing_limit_uses_agent_view_default_candidate_limit() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    seed_ready_cells(&mut db, 4);
    let query = r#"RETRIEVE CONTEXT FOR TASK "budget" IN BRAIN investment_projects
WHERE space = project:investments AND status = "ready";"#;

    let retrieved = db.retrieve_aql(query, &view(100, 3, 1_000, 400)).unwrap();

    assert_eq!(cell_ids(&retrieved), vec![CellId(1), CellId(2), CellId(3)]);
}

#[test]
fn aql_budget_drives_default_context_pack_token_budget() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    seed_ready_cells(&mut db, 1);
    let query = r#"RETRIEVE CONTEXT FOR TASK "budget" IN BRAIN investment_projects
WHERE space = project:investments AND status = "ready"
BUDGET 96 TOKENS LIMIT 10 CANDIDATES;"#;

    let pack = db
        .context_pack_from_aql(
            query,
            &view(100, 20, 1_000, 400),
            ContextPackOptions::default(),
        )
        .unwrap();

    assert_eq!(pack.token_budget_tokens, 96);
}

#[test]
fn aql_budget_is_policy_clamped_by_agent_view() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    seed_ready_cells(&mut db, 1);
    let query = r#"RETRIEVE CONTEXT FOR TASK "budget" IN BRAIN investment_projects
WHERE space = project:investments AND status = "ready"
BUDGET 1000 TOKENS LIMIT 10 CANDIDATES;"#;

    let pack = db
        .context_pack_from_aql(query, &view(100, 20, 64, 32), ContextPackOptions::default())
        .unwrap();

    assert_eq!(pack.token_budget_tokens, 64);
}

#[test]
fn explicit_context_pack_budget_overrides_aql_budget_but_stays_policy_clamped() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    seed_ready_cells(&mut db, 1);
    let query = r#"RETRIEVE CONTEXT FOR TASK "budget" IN BRAIN investment_projects
WHERE space = project:investments AND status = "ready"
BUDGET 96 TOKENS LIMIT 10 CANDIDATES;"#;

    let pack = db
        .context_pack_from_aql(
            query,
            &view(100, 20, 1_000, 400),
            ContextPackOptions {
                token_budget_tokens: 48,
                ..ContextPackOptions::default()
            },
        )
        .unwrap();

    assert_eq!(pack.token_budget_tokens, 48);
}

fn seed_ready_cells(db: &mut Database, count: u64) {
    for id in 1..=count {
        let payload = format!(
            "scope=project:investments\nstatus=ready\nsource=doc-{id}\nalpha budget cell {id}"
        );
        db.put_cell(CellId(id), payload.into_bytes()).unwrap();
    }
}

fn cell_ids(cells: &[cortex_engine::RetrievedCell]) -> Vec<CellId> {
    cells.iter().map(|cell| cell.cell_id).collect()
}

fn pack_cell_ids(pack: &cortex_engine::ContextPack) -> Vec<CellId> {
    pack.cells.iter().map(|cell| cell.cell_id).collect()
}

fn view(
    max_candidate_limit: u32,
    default_candidate_limit: u32,
    max_context_budget_tokens: u32,
    default_context_budget_tokens: u32,
) -> AgentView {
    AgentView {
        agent_id: AgentId(1),
        label: None,
        readable_brains: BTreeSet::from([BrainId(1)]),
        readable_scopes: BTreeSet::from([scope_id("project:investments")]),
        writable_scopes: BTreeSet::new(),
        allowed_modes: BTreeSet::from([RetrievalMode::Balanced]),
        allowed_memory_types: BTreeSet::from([MemoryType::Decision]),
        max_context_budget_tokens,
        default_context_budget_tokens,
        max_candidate_limit,
        default_candidate_limit,
        min_required_confidence_q16: Q16_ZERO,
        max_ttl_seconds: Some(3_600),
        allow_remember: false,
        allow_verify_fact: false,
        allow_audit_mode: false,
        require_citations_by_default: false,
        private_scope: None,
    }
}
