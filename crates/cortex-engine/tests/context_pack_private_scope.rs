use std::collections::BTreeSet;

use cortex_aql::{AgentId, AgentView, BrainId, MemoryType, RetrievalMode, Q16_ZERO};
use cortex_core::CellId;
use cortex_engine::{scope_id, ContextPackExportFormat, ContextPackOptions, Database};

const PUBLIC_SCOPE: &str = "project:investments";
const FORBIDDEN_SCOPE: &str = "agent:private";
const PRIVATE_SECRET: &str = "PRIVATE_SCOPE_SHOULD_NOT_LEAK";

#[test]
fn context_pack_broad_query_excludes_forbidden_scope_before_and_after_persistence() {
    let dir = tempfile::tempdir().unwrap();
    {
        let mut db = Database::open(dir.path()).unwrap();
        seed_public_and_private_ready_cells(&mut db);
        assert_no_private_scope_leak(&db);
        db.checkpoint().unwrap();
    }

    {
        let mut db = Database::open(dir.path()).unwrap();
        assert_no_private_scope_leak(&db);
        db.compact().unwrap();
    }

    let db = Database::open(dir.path()).unwrap();
    assert_no_private_scope_leak(&db);
}

#[test]
fn explicit_forbidden_scope_query_is_denied_before_packing() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    seed_public_and_private_ready_cells(&mut db);

    let error = db
        .context_pack_from_aql(
            private_scope_query(),
            &public_agent_view(),
            ContextPackOptions::default(),
        )
        .unwrap_err()
        .safe_message();

    assert_eq!(error, "requested scope is not readable");
}

#[test]
fn context_pack_records_access_decision_trail_per_cell() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    seed_public_and_private_ready_cells(&mut db);

    let pack = db
        .context_pack_from_aql(
            broad_ready_query(),
            &public_agent_view(),
            ContextPackOptions {
                token_budget_tokens: 256,
                ..ContextPackOptions::default()
            },
        )
        .unwrap();

    assert_eq!(pack.cells.len(), 1);
    let decision = pack.cells[0]
        .access_decision
        .as_ref()
        .expect("AQL ContextPack cells should carry access decisions");
    assert_eq!(decision.cell_id, CellId(1));
    assert_eq!(decision.decision.as_str(), "allowed");
    assert_eq!(decision.policy, "agent_view_readable_scope");
    assert_eq!(decision.scope, PUBLIC_SCOPE);
    assert_eq!(decision.scope_id, scope_id(PUBLIC_SCOPE).0);
    assert_eq!(decision.agent_id, Some(1));
}

#[test]
fn context_pack_acl_is_applied_before_candidate_limit() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_cell(
        CellId(1),
        format!("scope={FORBIDDEN_SCOPE}\nstatus=ready\nsource=private-source\n{PRIVATE_SECRET}")
            .into_bytes(),
    )
    .unwrap();
    db.put_cell(
        CellId(2),
        format!(
            "scope={PUBLIC_SCOPE}\nstatus=ready\nsource=public-source\npublic investment budget"
        )
        .into_bytes(),
    )
    .unwrap();

    assert_limit_one_returns_only_public_cell(&db);
    db.checkpoint().unwrap();
    assert_limit_one_returns_only_public_cell(&db);
}

fn seed_public_and_private_ready_cells(db: &mut Database) {
    db.put_cell(
        CellId(1),
        format!(
            "scope={PUBLIC_SCOPE}\nstatus=ready\nsource=public-source\npublic investment budget"
        )
        .into_bytes(),
    )
    .unwrap();
    db.put_cell(
        CellId(2),
        format!("scope={FORBIDDEN_SCOPE}\nstatus=ready\nsource=private-source\n{PRIVATE_SECRET}")
            .into_bytes(),
    )
    .unwrap();
}

fn assert_limit_one_returns_only_public_cell(db: &Database) {
    let retrieved = db
        .retrieve_aql(broad_ready_limit_one_query(), &public_agent_view())
        .unwrap();
    let ids = retrieved
        .iter()
        .map(|cell| cell.cell_id)
        .collect::<Vec<_>>();
    assert_eq!(ids, vec![CellId(2)]);

    let pack = db
        .context_pack_from_aql(
            broad_ready_limit_one_query(),
            &public_agent_view(),
            ContextPackOptions {
                token_budget_tokens: 256,
                ..ContextPackOptions::default()
            },
        )
        .unwrap();
    let pack_ids = pack
        .cells
        .iter()
        .map(|cell| cell.cell_id)
        .collect::<Vec<_>>();
    assert_eq!(pack_ids, vec![CellId(2)]);

    let surfaces = [
        pack.export(ContextPackExportFormat::Json),
        pack.export(ContextPackExportFormat::Prompt),
        pack.export(ContextPackExportFormat::Markdown),
    ];
    for surface in surfaces {
        assert!(surface.contains("public investment budget"));
        assert!(!surface.contains(PRIVATE_SECRET));
        assert!(!surface.contains(FORBIDDEN_SCOPE));
        assert!(!surface.contains("private-source"));
    }
}

fn assert_no_private_scope_leak(db: &Database) {
    let retrieved = db
        .retrieve_aql(broad_ready_query(), &public_agent_view())
        .unwrap();
    let ids = retrieved
        .iter()
        .map(|cell| cell.cell_id)
        .collect::<Vec<_>>();
    assert_eq!(ids, vec![CellId(1)]);

    let pack = db
        .context_pack_from_aql(
            broad_ready_query(),
            &public_agent_view(),
            ContextPackOptions {
                token_budget_tokens: 256,
                ..ContextPackOptions::default()
            },
        )
        .unwrap();
    let pack_ids = pack
        .cells
        .iter()
        .map(|cell| cell.cell_id)
        .collect::<Vec<_>>();
    assert_eq!(pack_ids, vec![CellId(1)]);

    let surfaces = [
        pack.export(ContextPackExportFormat::Json),
        pack.export(ContextPackExportFormat::Prompt),
        pack.export(ContextPackExportFormat::Markdown),
    ];
    for surface in surfaces {
        assert!(surface.contains("public investment budget"));
        assert!(!surface.contains(PRIVATE_SECRET));
        assert!(!surface.contains(FORBIDDEN_SCOPE));
        assert!(!surface.contains("private-source"));
    }
}

fn broad_ready_query() -> &'static str {
    r#"RETRIEVE CONTEXT FOR TASK "budget" IN BRAIN investment_projects
WHERE status = "ready" LIMIT 10 CANDIDATES;"#
}

fn broad_ready_limit_one_query() -> &'static str {
    r#"RETRIEVE CONTEXT FOR TASK "budget" IN BRAIN investment_projects
WHERE status = "ready" LIMIT 1 CANDIDATES;"#
}

fn private_scope_query() -> &'static str {
    r#"RETRIEVE CONTEXT FOR TASK "budget" IN BRAIN investment_projects
WHERE space = agent:private AND status = "ready" LIMIT 10 CANDIDATES;"#
}

fn public_agent_view() -> AgentView {
    AgentView {
        agent_id: AgentId(1),
        label: Some("public-agent".to_owned()),
        readable_brains: BTreeSet::from([BrainId(1)]),
        readable_scopes: BTreeSet::from([scope_id(PUBLIC_SCOPE)]),
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
        require_citations_by_default: false,
        private_scope: None,
    }
}
