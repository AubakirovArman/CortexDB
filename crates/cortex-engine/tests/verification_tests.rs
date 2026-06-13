use cortex_aql::{AgentId, AgentView, BrainId, MemoryType, RetrievalMode, Q16_ZERO};
use cortex_core::{CellId, KnowledgeCell, KnowledgeCellMetadata, KnowledgeCellType};
use cortex_engine::verification::ContradictionRelationOptions;
use cortex_engine::verification::{VerificationMatchKind, VerificationStatus};
use cortex_engine::{
    scope_id, Database, DatabaseOptions, PayloadResidency, SourceTrustCategory,
    INFERRED_SOURCE_TRUST_Q16,
};
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
    assert_eq!(
        report.evidence[0].match_kind,
        VerificationMatchKind::ExactText
    );
    assert_eq!(report.evidence[0].match_score_q16, u16::MAX);
}

#[test]
fn verify_fact_aql_rejects_partial_overlap_as_support() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_knowledge_cell(
        CellId(1),
        fact_cell("project:investments", "ABC budget delayed by committee"),
    )
    .unwrap();

    let report = db
        .verify_fact_aql(
            r#"VERIFY FACT "ABC budget approved" IN BRAIN investment_projects;"#,
            &view("project:investments", true),
        )
        .unwrap();

    assert_eq!(report.status, VerificationStatus::Insufficient);
    assert!(report.evidence.is_empty());
}

#[test]
fn verify_fact_aql_reports_semantic_entailment_match_kind() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_knowledge_cell(
        CellId(1),
        fact_cell(
            "project:investments",
            "Delta project remains active in registry",
        ),
    )
    .unwrap();

    let report = db
        .verify_fact_aql(
            r#"VERIFY FACT "Delta project active" IN BRAIN investment_projects;"#,
            &view("project:investments", true),
        )
        .unwrap();

    assert_eq!(report.status, VerificationStatus::Supported);
    assert_eq!(
        report.evidence[0].match_kind,
        VerificationMatchKind::SemanticEntailment
    );
    assert_eq!(report.evidence[0].match_score_q16, 60_000);
}

#[test]
fn verify_fact_aql_survives_restart() {
    let dir = tempfile::tempdir().unwrap();
    {
        let mut db = Database::open(dir.path()).unwrap();
        db.put_knowledge_cell(
            CellId(1),
            fact_cell("project:investments", "ABC budget approved"),
        )
        .unwrap();
    }

    let db = Database::open(dir.path()).unwrap();
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
fn verify_fact_aql_survives_lazy_checkpoint_reopen() {
    let dir = tempfile::tempdir().unwrap();
    {
        let mut db = Database::open(dir.path()).unwrap();
        db.put_knowledge_cell(
            CellId(1),
            fact_cell("project:investments", "ABC budget approved"),
        )
        .unwrap();
        db.checkpoint().unwrap();
    }
    let db = Database::open_with_options(
        dir.path(),
        DatabaseOptions {
            payload_residency: PayloadResidency::Lazy,
            ..DatabaseOptions::default()
        },
    )
    .unwrap();

    assert_eq!(db.storage_stats().unwrap().memtable_payload_bytes, 0);
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
fn verify_fact_aql_orders_equal_matches_by_source_trust() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_knowledge_cell(
        CellId(1),
        fact_cell_with_trust("project:investments", "ABC budget approved", Some(20_000)),
    )
    .unwrap();
    db.put_knowledge_cell(
        CellId(2),
        fact_cell_with_trust("project:investments", "ABC budget approved", Some(60_000)),
    )
    .unwrap();

    let report = db
        .verify_fact_aql(
            r#"VERIFY FACT "ABC budget approved" IN BRAIN investment_projects;"#,
            &view("project:investments", true),
        )
        .unwrap();
    assert_eq!(report.evidence[0].cell_id, CellId(2));
    assert_eq!(report.evidence[0].source_trust_q16, 60_000);
    assert_eq!(
        report.evidence[0].source_trust_category,
        SourceTrustCategory::Official
    );
}

#[test]
fn verify_fact_aql_reports_calibrated_report_confidence() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_knowledge_cell(
        CellId(1),
        fact_cell_with_trust("project:investments", "ABC budget approved", Some(60_000)),
    )
    .unwrap();

    let report = db
        .verify_fact_aql(
            r#"VERIFY FACT "ABC budget approved" IN BRAIN investment_projects;"#,
            &view("project:investments", true),
        )
        .unwrap();

    assert_eq!(report.status, VerificationStatus::Supported);
    assert_eq!(report.evidence[0].match_score_q16, u16::MAX);
    assert_eq!(report.evidence[0].source_trust_q16, 60_000);
    assert_eq!(report.confidence_q16, 60_000);
}

#[test]
fn verify_fact_aql_reports_source_trust_categories() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    for (cell_id, trust) in [
        (CellId(1), None),
        (CellId(2), Some(20_000)),
        (CellId(3), Some(32_768)),
        (CellId(4), Some(50_000)),
        (CellId(5), Some(60_000)),
    ] {
        db.put_knowledge_cell(
            cell_id,
            fact_cell_with_trust("project:investments", "ABC budget approved", trust),
        )
        .unwrap();
    }

    let report = db
        .verify_fact_aql(
            r#"VERIFY FACT "ABC budget approved" IN BRAIN investment_projects;"#,
            &view("project:investments", true),
        )
        .unwrap();
    let categories = report
        .evidence
        .iter()
        .map(|item| (item.cell_id, item.source_trust_category))
        .collect::<Vec<_>>();

    assert_eq!(
        categories,
        vec![
            (CellId(5), SourceTrustCategory::Official),
            (CellId(4), SourceTrustCategory::High),
            (CellId(1), SourceTrustCategory::Unknown),
            (CellId(3), SourceTrustCategory::Medium),
            (CellId(2), SourceTrustCategory::Low),
        ]
    );
}

#[test]
fn verify_fact_aql_uses_calibrated_source_trust_class() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_cell(
        CellId(1),
        b"scope=project:investments\nstatus=verified\ntype=fact\nsource=extracted-note\nsource_trust_class=inferred\n\nABC budget approved".to_vec(),
    )
    .unwrap();

    let report = db
        .verify_fact_aql(
            r#"VERIFY FACT "ABC budget approved" IN BRAIN investment_projects;"#,
            &view("project:investments", true),
        )
        .unwrap();

    assert_eq!(report.status, VerificationStatus::Supported);
    assert_eq!(
        report.evidence[0].source_trust_q16,
        INFERRED_SOURCE_TRUST_Q16
    );
    assert_eq!(
        report.evidence[0].source_trust_category,
        SourceTrustCategory::Low
    );
}

#[test]
fn verify_fact_aql_reports_contradicted_from_contradicts_line() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_knowledge_cell(
        CellId(1),
        fact_cell(
            "project:investments",
            "contradicts=ABC budget approved\nABC budget rejected",
        ),
    )
    .unwrap();

    let report = db
        .verify_fact_aql(
            r#"VERIFY FACT "ABC budget approved" IN BRAIN investment_projects;"#,
            &view("project:investments", true),
        )
        .unwrap();
    assert_eq!(report.status, VerificationStatus::Contradicted);
    assert!(report.evidence.is_empty());
    assert_eq!(report.contradicting_evidence[0].cell_id, CellId(1));
}

#[test]
fn verify_fact_aql_reports_mixed_when_support_and_contradiction_exist() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_knowledge_cell(
        CellId(1),
        fact_cell("project:investments", "ABC budget approved"),
    )
    .unwrap();
    db.put_knowledge_cell(
        CellId(2),
        fact_cell(
            "project:investments",
            "contradicts=ABC budget approved\nABC budget rejected",
        ),
    )
    .unwrap();

    let report = db
        .verify_fact_aql(
            r#"VERIFY FACT "ABC budget approved" IN BRAIN investment_projects;"#,
            &view("project:investments", true),
        )
        .unwrap();
    assert_eq!(report.status, VerificationStatus::Mixed);
    assert_eq!(report.evidence[0].cell_id, CellId(1));
    assert_eq!(report.contradicting_evidence[0].cell_id, CellId(2));
}

#[test]
fn conflict_index_lists_readable_contradiction_markers() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_knowledge_cell(
        CellId(1),
        fact_cell(
            "project:investments",
            "contradicts=ABC budget approved\nABC budget rejected",
        ),
    )
    .unwrap();
    db.put_knowledge_cell(
        CellId(2),
        fact_cell(
            "tenant:private",
            "contradicts=hidden budget approved\nhidden budget rejected",
        ),
    )
    .unwrap();

    let records = db.conflict_index(&view("project:investments", true));
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].cell_id, CellId(1));
    assert_eq!(records[0].fact, "ABC budget approved");
}

#[test]
fn conflicts_for_fact_filters_by_normalized_terms() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_knowledge_cell(
        CellId(1),
        fact_cell(
            "project:investments",
            "contradicts=ABC budget approved\nABC budget rejected",
        ),
    )
    .unwrap();
    db.put_knowledge_cell(
        CellId(2),
        fact_cell(
            "project:investments",
            "contradicts=XYZ budget approved\nXYZ budget rejected",
        ),
    )
    .unwrap();

    let records = db.conflicts_for_fact("abc budget approved", &view("project:investments", true));
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].cell_id, CellId(1));
}

#[test]
fn persisted_contradiction_relation_appears_in_conflict_index() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_knowledge_cell(
        CellId(1),
        fact_cell("project:investments", "ABC budget rejected"),
    )
    .unwrap();
    db.persist_contradiction_relation(
        CellId(10),
        CellId(1),
        "ABC budget approved",
        contradiction_relation_options("project:investments"),
    )
    .unwrap();

    let records = db.conflict_index(&view("project:investments", true));
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].cell_id, CellId(1));
    assert_eq!(records[0].relation_cell_id, Some(CellId(10)));
    assert_eq!(records[0].source_cell_id, Some(CellId(1)));
    assert_eq!(records[0].fact, "ABC budget approved");
}

#[test]
fn conflicts_for_fact_includes_persisted_relation() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.persist_contradiction_relation(
        CellId(10),
        CellId(1),
        "ABC budget approved",
        contradiction_relation_options("project:investments"),
    )
    .unwrap();

    let records = db.conflicts_for_fact("abc budget approved", &view("project:investments", true));
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].relation_cell_id, Some(CellId(10)));
}

#[test]
fn persisted_contradiction_relation_survives_restart() {
    let dir = tempfile::tempdir().unwrap();
    {
        let mut db = Database::open(dir.path()).unwrap();
        db.persist_contradiction_relation(
            CellId(10),
            CellId(1),
            "ABC budget approved",
            contradiction_relation_options("project:investments"),
        )
        .unwrap();
    }

    let db = Database::open(dir.path()).unwrap();
    let records = db.conflicts_for_fact("abc budget approved", &view("project:investments", true));
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].cell_id, CellId(1));
    assert_eq!(records[0].relation_cell_id, Some(CellId(10)));
}

#[test]
fn conflict_index_hides_unreadable_persisted_relation_scope() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.persist_contradiction_relation(
        CellId(10),
        CellId(1),
        "hidden budget approved",
        contradiction_relation_options("tenant:private"),
    )
    .unwrap();

    let records = db.conflict_index(&view("project:investments", true));
    assert!(records.is_empty());
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
    fact_cell_with_trust(scope, body, None)
}

fn fact_cell_with_trust(scope: &str, body: &str, trust: Option<u16>) -> KnowledgeCell {
    KnowledgeCell::new(
        KnowledgeCellMetadata {
            scope: scope.to_owned(),
            status: "verified".to_owned(),
            cell_type: KnowledgeCellType::Fact,
            memory_type: None,
            ttl_seconds: None,
            created_unix_seconds: None,
            source_trust_q16: trust,
            source: Some("fixture".to_owned()),
        },
        body,
    )
}

fn contradiction_relation_options(scope: &str) -> ContradictionRelationOptions {
    ContradictionRelationOptions {
        scope: scope.to_owned(),
        source: "fixture".to_owned(),
        source_trust_q16: Some(50_000),
    }
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
