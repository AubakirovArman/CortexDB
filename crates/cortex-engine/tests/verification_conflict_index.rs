use std::collections::BTreeSet;

use cortex_aql::{AgentId, AgentView, BrainId, MemoryType, RetrievalMode, Q16_ZERO};
use cortex_core::{CellId, KnowledgeCell, KnowledgeCellMetadata, KnowledgeCellType};
use cortex_engine::verification::ContradictionRelationOptions;
use cortex_engine::{scope_id, Database, DatabaseOptions, PayloadResidency};

#[test]
fn conflicts_for_entity_reads_structured_inline_marker() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_knowledge_cell(
        CellId(1),
        fact_cell(
            "project:investments",
            "ifc",
            "project=ABC Airport\nmetric=budget\ncontradicts=ABC Airport budget approved\nABC Airport budget rejected",
        ),
    )
    .unwrap();

    let records = db.conflicts_for_entity("abc airport", &view("project:investments"));
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].cell_id, CellId(1));
    assert_eq!(records[0].entity.as_deref(), Some("ABC Airport"));
    assert_eq!(records[0].metric.as_deref(), Some("budget"));
}

#[test]
fn conflicts_for_metric_filters_structured_markers() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_knowledge_cell(
        CellId(1),
        fact_cell(
            "project:investments",
            "ifc",
            "project=ABC Airport\nmetric=budget\ncontradicts=ABC Airport budget approved",
        ),
    )
    .unwrap();
    db.put_knowledge_cell(
        CellId(2),
        fact_cell(
            "project:investments",
            "ifc",
            "project=ABC Airport\nmetric=timeline\ncontradicts=ABC Airport completion 2029",
        ),
    )
    .unwrap();

    let records = db.conflicts_for_metric("budget", &view("project:investments"));
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].cell_id, CellId(1));
    assert_eq!(records[0].metric.as_deref(), Some("budget"));
}

#[test]
fn conflicts_for_source_filters_evidence_source() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_knowledge_cell(
        CellId(1),
        fact_cell(
            "project:investments",
            "ifc",
            "project=ABC Airport\nmetric=budget\ncontradicts=ABC Airport budget approved",
        ),
    )
    .unwrap();
    db.put_knowledge_cell(
        CellId(2),
        fact_cell(
            "project:investments",
            "world_bank",
            "project=XYZ Water\nmetric=budget\ncontradicts=XYZ Water budget approved",
        ),
    )
    .unwrap();

    let records = db.conflicts_for_source("ifc", &view("project:investments"));
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].cell_id, CellId(1));
    assert_eq!(records[0].source.as_deref(), Some("ifc"));
}

#[test]
fn persisted_relation_can_be_queried_by_source_cell_facets() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_knowledge_cell(
        CellId(1),
        fact_cell(
            "project:investments",
            "ifc",
            "project=ABC Airport\nmetric=budget\nABC Airport budget rejected",
        ),
    )
    .unwrap();
    db.persist_contradiction_relation(
        CellId(10),
        CellId(1),
        "ABC Airport budget approved",
        contradiction_relation_options("project:investments"),
    )
    .unwrap();

    let by_entity = db.conflicts_for_entity("abc airport", &view("project:investments"));
    assert_eq!(by_entity.len(), 1);
    assert_eq!(by_entity[0].relation_cell_id, Some(CellId(10)));
    assert_eq!(by_entity[0].entity.as_deref(), Some("ABC Airport"));

    let by_metric = db.conflicts_for_metric("budget", &view("project:investments"));
    assert_eq!(by_metric.len(), 1);
    assert_eq!(by_metric[0].relation_cell_id, Some(CellId(10)));

    let by_source = db.conflicts_for_source("ifc", &view("project:investments"));
    assert_eq!(by_source.len(), 1);
    assert_eq!(by_source[0].relation_cell_id, Some(CellId(10)));
    assert_eq!(by_source[0].source.as_deref(), Some("ifc"));
}

#[test]
fn persisted_relation_can_be_queried_by_source_cell_facets_lazy_checkpoint_reopen() {
    let dir = tempfile::tempdir().unwrap();
    {
        let mut db = Database::open(dir.path()).unwrap();
        db.put_knowledge_cell(
            CellId(1),
            fact_cell(
                "project:investments",
                "ifc",
                "project=ABC Airport\nmetric=budget\nABC Airport budget rejected",
            ),
        )
        .unwrap();
        db.persist_contradiction_relation(
            CellId(10),
            CellId(1),
            "ABC Airport budget approved",
            contradiction_relation_options("project:investments"),
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
    let by_entity = db.conflicts_for_entity("abc airport", &view("project:investments"));

    assert_eq!(by_entity.len(), 1);
    assert_eq!(by_entity[0].relation_cell_id, Some(CellId(10)));
    assert_eq!(by_entity[0].entity.as_deref(), Some("ABC Airport"));
    assert_eq!(by_entity[0].metric.as_deref(), Some("budget"));
    assert_eq!(by_entity[0].source.as_deref(), Some("ifc"));
}

#[test]
fn persisted_relation_does_not_use_hidden_source_cell_facets() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_knowledge_cell(
        CellId(1),
        fact_cell(
            "tenant:private",
            "ifc",
            "project=Hidden Airport\nmetric=budget\nHidden Airport budget rejected",
        ),
    )
    .unwrap();
    db.persist_contradiction_relation(
        CellId(10),
        CellId(1),
        "Hidden Airport budget approved",
        contradiction_relation_options("project:investments"),
    )
    .unwrap();

    let records = db.conflicts_for_source("ifc", &view("project:investments"));
    assert!(records.is_empty());
}

fn fact_cell(scope: &str, source: &str, body: &str) -> KnowledgeCell {
    KnowledgeCell::new(
        KnowledgeCellMetadata {
            scope: scope.to_owned(),
            status: "verified".to_owned(),
            cell_type: KnowledgeCellType::Fact,
            memory_type: None,
            ttl_seconds: None,
            created_unix_seconds: None,
            source_trust_q16: Some(50_000),
            source: Some(source.to_owned()),
        },
        body,
    )
}

fn contradiction_relation_options(scope: &str) -> ContradictionRelationOptions {
    ContradictionRelationOptions {
        scope: scope.to_owned(),
        source: "reviewer".to_owned(),
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
