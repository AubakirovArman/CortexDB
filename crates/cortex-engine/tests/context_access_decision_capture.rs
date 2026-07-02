use std::collections::BTreeSet;

use cortex_aql::{AgentId, AgentView, BrainId, MemoryType, RetrievalMode, Q16_ZERO};
use cortex_core::CellId;
use cortex_engine::{
    scope_id, ContextAccessDecisionOutcome, ContextPackExportFormat, ContextPackOptions, Database,
};
use serde_json::Value;

const PUBLIC_SCOPE: &str = "project:access-capture";
const PRIVATE_SCOPE: &str = "project:access-denied";

#[test]
fn aql_context_pack_uses_captured_access_decision_from_retrieval_path() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    seed(&mut db);
    let view = view();
    let query = r#"RETRIEVE CONTEXT FOR TASK "budget evidence" IN BRAIN investment_projects
WHERE status = "ready" LIMIT 10 CANDIDATES;"#;

    let retrieved = db.retrieve_aql(query, &view).unwrap();
    assert_eq!(retrieved.len(), 1);
    assert_eq!(retrieved[0].cell_id, CellId(1));
    let captured = retrieved[0]
        .captured_access_decision
        .as_ref()
        .expect("AQL retrieval should capture access before packing");
    assert_eq!(captured.policy, "agent_view_readable_scope");
    assert_eq!(captured.policy_version, "agent_view_readable_scope.v1");
    assert_eq!(captured.scope, PUBLIC_SCOPE);
    assert_eq!(captured.scope_id, scope_id(PUBLIC_SCOPE).0);
    assert_eq!(captured.agent_id, Some(17));
    assert_eq!(captured.agent_view_digest.len(), 64);
    assert!(captured
        .reason
        .contains("survived AQL permission filtering"));

    let pack = db
        .context_pack_from_aql(
            query,
            &view,
            ContextPackOptions {
                token_budget_tokens: 512,
                ..ContextPackOptions::default()
            },
        )
        .unwrap();
    assert_eq!(pack.cells.len(), 1);
    let decision = pack.cells[0]
        .access_decision
        .as_ref()
        .expect("packed AQL cells should carry captured access decisions");
    assert_eq!(decision.cell_id, CellId(1));
    assert_eq!(decision.decision, ContextAccessDecisionOutcome::Allowed);
    assert_eq!(decision.policy, "agent_view_readable_scope");
    assert_eq!(
        decision.policy_version.as_deref(),
        Some("agent_view_readable_scope.v1")
    );
    assert_eq!(decision.scope, PUBLIC_SCOPE);
    assert_eq!(decision.scope_id, scope_id(PUBLIC_SCOPE).0);
    assert_eq!(decision.agent_id, Some(17));
    assert_eq!(
        decision.agent_view_digest.as_deref(),
        Some(captured.agent_view_digest.as_str())
    );
    assert!(decision
        .reason
        .contains("survived AQL permission filtering"));
    assert!(!decision.reason.contains("re-derived"));

    let exported: Value =
        serde_json::from_str(&pack.export(ContextPackExportFormat::Json)).unwrap();
    let exported_decision = &exported["cells"][0]["access_decision"];
    assert_eq!(exported_decision["decision"], "allowed");
    assert_eq!(
        exported_decision["policy_version"],
        "agent_view_readable_scope.v1"
    );
    assert_eq!(
        exported_decision["agent_view_digest"],
        captured.agent_view_digest
    );
}

fn seed(db: &mut Database) {
    db.put_cell(
        CellId(1),
        format!("scope={PUBLIC_SCOPE}\nstatus=ready\nsource=public\n\nPublic budget evidence")
            .into_bytes(),
    )
    .unwrap();
    db.put_cell(
        CellId(2),
        format!("scope={PRIVATE_SCOPE}\nstatus=ready\nsource=private\n\nPrivate budget evidence")
            .into_bytes(),
    )
    .unwrap();
}

fn view() -> AgentView {
    AgentView {
        agent_id: AgentId(17),
        label: Some("access-capture".to_owned()),
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
