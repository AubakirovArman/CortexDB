use cortex_aql::{AgentId, AgentView, BrainId, MemoryType, RetrievalMode, Q16_ZERO};
use cortex_core::{CellId, KnowledgeCell, KnowledgeCellMetadata, KnowledgeCellType};
use cortex_engine::verification::VerificationStatus;
use cortex_engine::{scope_id, Database};
use std::collections::BTreeSet;

#[test]
fn verify_fact_reports_contradicted_from_negated_sentence() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_knowledge_cell(
        CellId(1),
        fact_cell(
            "project:investments",
            "ABC budget was not approved by committee",
        ),
    )
    .unwrap();

    let report = db
        .verify_fact_aql(
            r#"VERIFY FACT "ABC budget approved" IN BRAIN investment_projects;"#,
            &view("project:investments"),
        )
        .unwrap();

    assert_eq!(report.status, VerificationStatus::Contradicted);
    assert!(report.evidence.is_empty());
    assert_eq!(report.contradicting_evidence[0].cell_id, CellId(1));
    assert_eq!(report.contradicting_evidence[0].matched_terms, 3);
}

#[test]
fn verify_fact_reports_contradicted_from_antonym_sentence() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_knowledge_cell(
        CellId(1),
        fact_cell(
            "project:investments",
            "The committee rejected the ABC budget after review",
        ),
    )
    .unwrap();

    let report = db
        .verify_fact_aql(
            r#"VERIFY FACT "ABC budget approved" IN BRAIN investment_projects;"#,
            &view("project:investments"),
        )
        .unwrap();

    assert_eq!(report.status, VerificationStatus::Contradicted);
    assert!(report.evidence.is_empty());
    assert_eq!(report.contradicting_evidence[0].cell_id, CellId(1));
}

#[test]
fn verify_fact_reports_mixed_with_natural_language_contradiction() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_knowledge_cell(
        CellId(1),
        fact_cell("project:investments", "ABC budget approved by board"),
    )
    .unwrap();
    db.put_knowledge_cell(
        CellId(2),
        fact_cell("project:investments", "ABC budget is not approved for 2025"),
    )
    .unwrap();

    let report = db
        .verify_fact_aql(
            r#"VERIFY FACT "ABC budget approved" IN BRAIN investment_projects;"#,
            &view("project:investments"),
        )
        .unwrap();

    assert_eq!(report.status, VerificationStatus::Mixed);
    assert_eq!(report.evidence[0].cell_id, CellId(1));
    assert_eq!(report.contradicting_evidence[0].cell_id, CellId(2));
}

#[test]
fn verify_fact_does_not_treat_not_only_as_contradiction() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_knowledge_cell(
        CellId(1),
        fact_cell(
            "project:investments",
            "ABC budget was not only approved but also funded",
        ),
    )
    .unwrap();

    let report = db
        .verify_fact_aql(
            r#"VERIFY FACT "ABC budget approved" IN BRAIN investment_projects;"#,
            &view("project:investments"),
        )
        .unwrap();

    assert_eq!(report.status, VerificationStatus::Supported);
    assert_eq!(report.evidence[0].cell_id, CellId(1));
    assert!(report.contradicting_evidence.is_empty());
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
            source_trust_q16: None,
            source: Some("fixture".to_owned()),
        },
        body,
    )
}

fn view(scope: &str) -> AgentView {
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
        allow_verify_fact: true,
        allow_audit_mode: false,
        require_citations_by_default: false,
        private_scope: None,
    }
}
