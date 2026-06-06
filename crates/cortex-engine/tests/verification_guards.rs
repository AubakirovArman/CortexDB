use cortex_aql::{AgentId, AgentView, BrainId, MemoryType, RetrievalMode, Q16_ZERO};
use cortex_core::{CellId, KnowledgeCell, KnowledgeCellMetadata, KnowledgeCellType};
use cortex_engine::verification::{
    VerificationGuardCode, VerificationReportExportFormat, VerificationStatus,
};
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
    assert_eq!(
        report.guards[0].code,
        VerificationGuardCode::MissingCitation
    );
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
    assert_eq!(
        report.guards[0].code,
        VerificationGuardCode::NumericMismatch
    );
    assert_eq!(report.numeric_conflicts.len(), 1);
    assert_eq!(report.numeric_conflicts[0].cell_id, CellId(1));
    assert_eq!(report.numeric_conflicts[0].metric, "metric");
    assert_eq!(report.numeric_conflicts[0].left, "12000");
    assert_eq!(report.numeric_conflicts[0].right, "13000");
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
    assert_eq!(
        report.guards[0].code,
        VerificationGuardCode::NumericMismatch
    );
}

#[test]
fn verification_report_contains_structured_numeric_conflict() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_cell(
        CellId(7),
        b"scope=project:investments\nstatus=verified\ntype=fact\nsource=annual-report\nmetric=budget\n\nSolar Plant budget increased to 1.4B KZT.".to_vec(),
    )
    .unwrap();

    let report = db
        .verify_fact_aql(
            r#"VERIFY FACT "Solar Plant budget is 1.2B KZT" IN BRAIN investment_projects;"#,
            &view(),
        )
        .unwrap();

    assert_eq!(report.status, VerificationStatus::Contradicted);
    assert_eq!(report.numeric_conflicts.len(), 1);
    let conflict = &report.numeric_conflicts[0];
    assert_eq!(conflict.cell_id, CellId(7));
    assert_eq!(conflict.metric, "budget");
    assert_eq!(conflict.left, "1.2B KZT");
    assert_eq!(conflict.right, "1.4B KZT");
    assert_eq!(conflict.fact_value.scaled_value, 1_200_000_000);
    assert_eq!(conflict.evidence_value.scaled_value, 1_400_000_000);
}

#[test]
fn verification_report_does_not_infer_billions_from_decimal_percent() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_cell(
        CellId(9),
        b"scope=project:investments\nstatus=verified\ntype=fact\nsource=risk-report\nmetric=risk\n\nProject risk changed to 2%.".to_vec(),
    )
    .unwrap();

    let report = db
        .verify_fact_aql(
            r#"VERIFY FACT "Project risk is 1.2%" IN BRAIN investment_projects;"#,
            &view(),
        )
        .unwrap();

    assert_eq!(report.status, VerificationStatus::Contradicted);
    assert_eq!(report.numeric_conflicts.len(), 1);
    assert_eq!(report.numeric_conflicts[0].metric, "risk");
    assert_eq!(report.numeric_conflicts[0].left, "1.2 %");
    assert_eq!(report.numeric_conflicts[0].right, "2 %");
    assert!(!report.numeric_conflicts[0].left.contains("1.2B"));
}

#[test]
fn stale_valid_to_evidence_is_guarded_and_not_supporting() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_cell(
        CellId(11),
        b"scope=project:investments\nstatus=verified\ntype=fact\nsource=archive\nvalid_to=2024-12-31\n\nSolar Plant budget is 1.2B KZT.".to_vec(),
    )
    .unwrap();

    let report = db
        .verify_fact_aql(
            r#"VERIFY FACT "Solar Plant budget is 1.2B KZT on 2025-01-10" IN BRAIN investment_projects;"#,
            &view(),
        )
        .unwrap();

    assert_eq!(report.status, VerificationStatus::Insufficient);
    assert!(report.evidence.is_empty());
    assert!(report.contradicting_evidence.is_empty());
    assert_eq!(report.guards.len(), 1);
    assert_eq!(report.guards[0].code, VerificationGuardCode::StaleFact);
    assert!(report.guards[0].message.contains("valid_to=2024-12-31"));
}

#[test]
fn stale_evidence_does_not_create_numeric_contradiction() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_cell(
        CellId(12),
        b"scope=project:investments\nstatus=verified\ntype=fact\nsource=archive\nmetric=budget\nvalid_to=2024-12-31\n\nSolar Plant budget is 1.4B KZT.".to_vec(),
    )
    .unwrap();

    let report = db
        .verify_fact_aql(
            r#"VERIFY FACT "Solar Plant budget is 1.2B KZT on 2025-01-10" IN BRAIN investment_projects;"#,
            &view(),
        )
        .unwrap();

    assert_eq!(report.status, VerificationStatus::Insufficient);
    assert!(report.contradicting_evidence.is_empty());
    assert!(report.numeric_conflicts.is_empty());
    assert_eq!(report.guards[0].code, VerificationGuardCode::StaleFact);
}

#[test]
fn future_valid_from_evidence_is_guarded_until_valid() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_cell(
        CellId(13),
        b"scope=project:investments\nstatus=verified\ntype=fact\nsource=forward-plan\nvalid_from=2026-01-01\n\nSolar Plant budget is 1.2B KZT.".to_vec(),
    )
    .unwrap();

    let report = db
        .verify_fact_aql(
            r#"VERIFY FACT "Solar Plant budget is 1.2B KZT in 2025" IN BRAIN investment_projects;"#,
            &view(),
        )
        .unwrap();

    assert_eq!(report.status, VerificationStatus::Insufficient);
    assert_eq!(report.guards[0].code, VerificationGuardCode::StaleFact);
    assert!(report.guards[0].message.contains("valid_from=2026-01-01"));
}

#[test]
fn evidence_inside_validity_window_supports_fact() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_cell(
        CellId(14),
        b"scope=project:investments\nstatus=verified\ntype=fact\nsource=annual-report\nvalid_from=2025-01-01\nvalid_to=2025-12-31\n\nSolar Plant budget is 1.2B KZT.".to_vec(),
    )
    .unwrap();

    let report = db
        .verify_fact_aql(
            r#"VERIFY FACT "Solar Plant budget is 1.2B KZT on 2025-05-01" IN BRAIN investment_projects;"#,
            &view(),
        )
        .unwrap();

    assert_eq!(report.status, VerificationStatus::Supported);
    assert_eq!(report.evidence[0].cell_id, CellId(14));
    assert!(report.guards.is_empty());
}

#[test]
fn verification_report_exports_markdown_and_audit_text() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_cell(
        CellId(7),
        b"scope=project:investments\nstatus=verified\ntype=fact\nsource=annual-report\nsource_trust_q16=60000\nmetric=budget\n\nSolar Plant budget increased to 1.4B KZT.".to_vec(),
    )
    .unwrap();

    let report = db
        .verify_fact_aql(
            r#"VERIFY FACT "Solar Plant budget is 1.2B KZT" IN BRAIN investment_projects;"#,
            &view(),
        )
        .unwrap();
    let markdown = report.export(VerificationReportExportFormat::Markdown);
    let audit = report.export(VerificationReportExportFormat::Audit);

    assert!(markdown.starts_with("# CortexDB Verification Report"));
    assert!(markdown.contains("## Numeric Conflicts"));
    assert!(markdown.contains("metric=`budget`"));
    assert!(markdown.contains("source_trust=`official` (`60000`)"));
    assert!(audit.starts_with("CortexDB Verification Audit v1"));
    assert!(audit.contains("status=contradicted"));
    assert!(audit.contains("numeric_conflict.0.metric=budget"));
    assert!(audit.contains("contradicting.0.source_trust_category=official"));
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
