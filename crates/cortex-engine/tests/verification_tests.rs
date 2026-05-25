use cortex_aql::{AgentId, AgentView, BrainId, MemoryType, RetrievalMode, Q16_ZERO};
use cortex_core::{CellId, KnowledgeCell, KnowledgeCellMetadata, KnowledgeCellType};
use cortex_engine::verification::VerificationStatus;
use cortex_engine::{scope_id, Database};
use std::collections::BTreeSet;

#[test]
fn verify_fact_aql_reports_supported_evidence() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_knowledge_cell(
        CellId(1),
        fact_cell("project:investments", "ABC budget approved"),
    )
    .unwrap();

    let report = db
        .verify_fact_aql(
            r#"VERIFY FACT "ABC budget approved" IN BRAIN investment_projects;"#,
            &view("project:investments", true),
        )
        .unwrap();
    assert_eq!(report.status, VerificationStatus::Supported);
    assert_eq!(report.evidence[0].cell_id, CellId(1));
}

#[test]
fn verify_fact_aql_respects_agent_scope() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_knowledge_cell(
        CellId(1),
        fact_cell("tenant:private", "hidden budget approved"),
    )
    .unwrap();

    let report = db
        .verify_fact_aql(
            r#"VERIFY FACT "hidden budget approved" IN BRAIN investment_projects;"#,
            &view("project:investments", true),
        )
        .unwrap();
    assert_eq!(report.status, VerificationStatus::Insufficient);
    assert!(report.evidence.is_empty());
}

#[test]
fn verify_fact_aql_denied_by_agent_view_policy() {
    let dir = tempfile::tempdir().unwrap();
    let db = Database::open(dir.path()).unwrap();
    let error = db
        .verify_fact_aql(
            r#"VERIFY FACT "ABC budget approved" IN BRAIN investment_projects;"#,
            &view("project:investments", false),
        )
        .unwrap_err();
    assert!(error.to_string().contains("VerifyFactNotAllowed"));
}

fn fact_cell(scope: &str, body: &str) -> KnowledgeCell {
    KnowledgeCell::new(
        KnowledgeCellMetadata {
            scope: scope.to_owned(),
            status: "verified".to_owned(),
            cell_type: KnowledgeCellType::Fact,
            memory_type: None,
            ttl_seconds: None,
            created_unix_seconds: None,
            source: Some("fixture".to_owned()),
        },
        body,
    )
}

fn view(scope: &str, allow_verify: bool) -> AgentView {
    AgentView {
        agent_id: AgentId(1),
        label: None,
        readable_brains: BTreeSet::from([BrainId(1)]),
        readable_scopes: BTreeSet::from([scope_id(scope)]),
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
        allow_verify_fact: allow_verify,
        allow_audit_mode: false,
        require_citations_by_default: false,
        private_scope: None,
    }
}
