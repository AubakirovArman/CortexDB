use cortex_aql::{AgentId, AgentView, BrainId, MemoryType, RetrievalMode, Q16_ZERO};
use cortex_core::{CellId, KnowledgeCell, KnowledgeCellMetadata, KnowledgeCellType};
use cortex_engine::verification::VerificationStatus;
use cortex_engine::{scope_id, Database};
use std::collections::BTreeSet;

#[test]
fn verify_fact_reports_missing_citation_guard() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_knowledge_cell(CellId(1), fact_cell(None, "ABC budget approved"))
        .unwrap();

    let report = db
        .verify_fact_aql(
            r#"VERIFY FACT "ABC budget approved" IN BRAIN investment_projects;"#,
            &view(),
        )
        .unwrap();

    assert_eq!(report.status, VerificationStatus::Supported);
    assert_eq!(report.guards[0].code, "missing_citation");
    assert_eq!(report.guards[0].cell_id, Some(CellId(1)));
}

#[test]
fn verify_fact_reports_numeric_mismatch_guard_as_contradiction() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_knowledge_cell(
        CellId(1),
        fact_cell(Some("annual-report"), "ABC budget approved for 13000"),
    )
    .unwrap();

    let report = db
        .verify_fact_aql(
            r#"VERIFY FACT "ABC budget approved for 12000" IN BRAIN investment_projects;"#,
            &view(),
        )
        .unwrap();

    assert_eq!(report.status, VerificationStatus::Contradicted);
    assert!(report.evidence.is_empty());
    assert_eq!(report.contradicting_evidence[0].cell_id, CellId(1));
    assert_eq!(report.guards[0].code, "numeric_mismatch");
}

#[test]
fn verify_fact_accepts_matching_normalized_numbers() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_knowledge_cell(
        CellId(1),
        fact_cell(Some("annual-report"), "ABC budget approved for 12000.00"),
    )
    .unwrap();

    let report = db
        .verify_fact_aql(
            r#"VERIFY FACT "ABC budget approved for 12,000" IN BRAIN investment_projects;"#,
            &view(),
        )
        .unwrap();

    assert_eq!(report.status, VerificationStatus::Supported);
    assert!(report.guards.is_empty());
}

#[test]
fn verify_fact_reports_numeric_mismatch_even_with_shared_year() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_knowledge_cell(
        CellId(1),
        fact_cell(
            Some("annual-report"),
            "ABC budget approved for 9000 in 2025",
        ),
    )
    .unwrap();

    let report = db
        .verify_fact_aql(
            r#"VERIFY FACT "ABC budget approved for 12000 in 2025" IN BRAIN investment_projects;"#,
            &view(),
        )
        .unwrap();

    assert_eq!(report.status, VerificationStatus::Contradicted);
    assert_eq!(report.guards[0].code, "numeric_mismatch");
}

fn fact_cell(source: Option<&str>, body: &str) -> KnowledgeCell {
    KnowledgeCell::new(
        KnowledgeCellMetadata {
            scope: "project:investments".to_owned(),
            status: "verified".to_owned(),
            cell_type: KnowledgeCellType::Fact,
            memory_type: None,
            ttl_seconds: None,
            created_unix_seconds: None,
            source_trust_q16: None,
            source: source.map(str::to_owned),
        },
        body,
    )
}

fn view() -> AgentView {
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
        allow_verify_fact: true,
        allow_audit_mode: false,
        require_citations_by_default: false,
        private_scope: None,
    }
}
