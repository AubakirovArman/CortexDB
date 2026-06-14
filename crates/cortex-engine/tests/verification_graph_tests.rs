use std::collections::BTreeSet;

use cortex_aql::{AgentId, AgentView, BrainId, MemoryType, RetrievalMode, Q16_ZERO};
use cortex_core::{CellId, KnowledgeCell, KnowledgeCellMetadata, KnowledgeCellType};
use cortex_engine::verification::{ContradictionRelationOptions, VerificationStatus};
use cortex_engine::{scope_id, Database, DatabaseOptions, PayloadResidency, SourceTrustCategory};

fn fact_cell(scope: &str, body: &str, trust: Option<u16>) -> KnowledgeCell {
    KnowledgeCell::new(
        KnowledgeCellMetadata {
            scope: scope.to_owned(),
            status: "verified".to_owned(),
            cell_type: KnowledgeCellType::Fact,
            source_trust_q16: trust,
            ..KnowledgeCellMetadata::default()
        },
        body,
    )
}

fn source_support_relation(scope: &str, fact_cell_id: CellId, trust: Option<u16>) -> KnowledgeCell {
    KnowledgeCell::new(
        KnowledgeCellMetadata {
            scope: scope.to_owned(),
            status: "ready".to_owned(),
            cell_type: KnowledgeCellType::Relation,
            source: Some("ifc:disclosure-001".to_owned()),
            source_trust_q16: trust,
            ..KnowledgeCellMetadata::default()
        },
        format!(
            "subject=source:ifc:disclosure-001\npredicate=source_supports_fact\nobject=cell:{}",
            fact_cell_id.0
        ),
    )
}

#[test]
fn verify_fact_aql_uses_relation_graph_contradiction() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.persist_contradiction_relation(
        CellId(10),
        CellId(99),
        "ABC budget approved",
        contradiction_relation_options("project:investments"),
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
    assert_eq!(report.contradicting_evidence[0].cell_id, CellId(10));
}

#[test]
fn verify_fact_aql_enriches_evidence_from_source_support_edge() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_knowledge_cell(
        CellId(1),
        fact_cell("project:investments", "ABC budget approved", Some(20_000)),
    )
    .unwrap();
    db.put_knowledge_cell(
        CellId(10),
        source_support_relation("project:investments", CellId(1), Some(60_000)),
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
    assert_eq!(
        report.evidence[0].citation,
        Some("ifc:disclosure-001".to_owned())
    );
    assert_eq!(report.evidence[0].source_trust_q16, 60_000);
    assert_eq!(
        report.evidence[0].source_trust_category,
        SourceTrustCategory::Official
    );
}

#[test]
fn verify_fact_aql_enriches_evidence_from_source_support_edge_lazy_checkpoint_reopen() {
    let dir = tempfile::tempdir().unwrap();
    {
        let mut db = Database::open(dir.path()).unwrap();
        db.put_knowledge_cell(
            CellId(1),
            fact_cell("project:investments", "ABC budget approved", Some(20_000)),
        )
        .unwrap();
        db.put_knowledge_cell(
            CellId(10),
            source_support_relation("project:investments", CellId(1), Some(60_000)),
        )
        .unwrap();
        db.put_knowledge_cell(
            CellId(11),
            source_support_relation("project:investments", CellId(99), Some(65_000)),
        )
        .unwrap();
        db.checkpoint().unwrap();
    }
    let db = Database::open_with_options(
        dir.path(),
        DatabaseOptions {
            payload_residency: PayloadResidency::Lazy,
            payload_cache_bytes: 0,
            ..DatabaseOptions::default()
        },
    )
    .unwrap();

    assert_eq!(db.storage_stats().unwrap().memtable_payload_bytes, 0);
    let report = db
        .verify_fact_aql(
            r#"VERIFY FACT "ABC budget approved" IN BRAIN investment_projects;"#,
            &view("project:investments"),
        )
        .unwrap();

    assert_eq!(report.status, VerificationStatus::Supported);
    assert_eq!(report.evidence[0].cell_id, CellId(1));
    assert_eq!(
        report.evidence[0].citation,
        Some("ifc:disclosure-001".to_owned())
    );
    assert_eq!(report.evidence[0].source_trust_q16, 60_000);
    assert_eq!(
        db.payload_cache_stats().segment_loads,
        2,
        "VERIFY should read the fact and matching source-support relation only"
    );
}

#[test]
fn verify_fact_aql_ignores_unreadable_source_support_edge() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_knowledge_cell(
        CellId(1),
        fact_cell("project:investments", "ABC budget approved", Some(20_000)),
    )
    .unwrap();
    db.put_knowledge_cell(
        CellId(10),
        source_support_relation("tenant:private", CellId(1), Some(60_000)),
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
    assert_eq!(report.evidence[0].citation, None);
    assert_eq!(report.evidence[0].source_trust_q16, 20_000);
}

#[test]
fn verify_fact_aql_checks_persisted_source_support_descriptor_before_lazy_payload_read() {
    let dir = tempfile::tempdir().unwrap();
    {
        let mut db = Database::open(dir.path()).unwrap();
        db.put_knowledge_cell(
            CellId(1),
            fact_cell("project:investments", "ABC budget approved", Some(20_000)),
        )
        .unwrap();
        db.put_knowledge_cell(
            CellId(10),
            source_support_relation("tenant:private", CellId(1), Some(60_000)),
        )
        .unwrap();
        db.checkpoint().unwrap();
    }
    let db = Database::open_with_options(
        dir.path(),
        DatabaseOptions {
            payload_residency: PayloadResidency::Lazy,
            payload_cache_bytes: 0,
            ..DatabaseOptions::default()
        },
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
    assert_eq!(report.evidence[0].citation, None);
    assert_eq!(report.evidence[0].source_trust_q16, 20_000);
    assert_eq!(
        db.payload_cache_stats().segment_loads,
        1,
        "verification should not read unreadable source-support relation payload"
    );
}

fn contradiction_relation_options(scope: &str) -> ContradictionRelationOptions {
    ContradictionRelationOptions {
        scope: scope.to_owned(),
        source: "fixture".to_owned(),
        source_trust_q16: Some(50_000),
    }
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
