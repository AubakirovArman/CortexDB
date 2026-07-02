use std::collections::BTreeSet;

use cortex_aql::{AgentId, AgentView, BrainId, MemoryType, RetrievalMode, Q16_ZERO};
use cortex_core::CellId;
use cortex_engine::{scope_id, Database, DatabaseOptions, PayloadResidency};

#[test]
fn numeric_conflict_index_tracks_write_patch_tombstone_and_lazy_reopen() {
    let dir = tempfile::tempdir().unwrap();
    {
        let mut db = Database::open(dir.path()).unwrap();
        db.put_cell(
            CellId(1),
            numeric_fact_payload("project:investments", "ifc", "Mirny", "budget", "1.2B KZT"),
        )
        .unwrap();
        db.put_cell(
            CellId(2),
            numeric_fact_payload(
                "project:investments",
                "world_bank",
                "Mirny",
                "budget",
                "1.4B KZT",
            ),
        )
        .unwrap();

        let records = db.conflicts_for_metric("budget", &view("project:investments"));
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].cell_id, CellId(1));
        assert_eq!(records[0].source_cell_id, Some(CellId(2)));
        assert_eq!(records[0].entity.as_deref(), Some("Mirny"));
        assert_eq!(records[0].metric.as_deref(), Some("budget"));

        db.patch_cell(
            CellId(2),
            numeric_fact_payload(
                "project:investments",
                "world_bank",
                "Mirny",
                "budget",
                "1.2B KZT",
            ),
        )
        .unwrap();
        assert!(db
            .conflicts_for_metric("budget", &view("project:investments"))
            .is_empty());

        db.patch_cell(
            CellId(2),
            numeric_fact_payload(
                "project:investments",
                "world_bank",
                "Mirny",
                "budget",
                "1.6B KZT",
            ),
        )
        .unwrap();
        assert_eq!(
            db.conflicts_for_metric("budget", &view("project:investments"))
                .len(),
            1
        );

        db.tombstone_cell(CellId(2)).unwrap();
        assert!(db
            .conflicts_for_metric("budget", &view("project:investments"))
            .is_empty());

        db.put_cell(
            CellId(2),
            numeric_fact_payload(
                "project:investments",
                "world_bank",
                "Mirny",
                "budget",
                "1.4B KZT",
            ),
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
    let records = db.conflicts_for_metric("budget", &view("project:investments"));
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].source_cell_id, Some(CellId(2)));
}

#[test]
fn temporal_numeric_conflict_index_respects_overlapping_windows() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_cell(
        CellId(11),
        numeric_temporal_fact_payload(
            "project:investments",
            "ifc",
            "Mirny",
            "budget",
            "1.2B KZT",
            Some("2025-01-01"),
            Some("2025-06-30"),
        ),
    )
    .unwrap();
    db.put_cell(
        CellId(12),
        numeric_temporal_fact_payload(
            "project:investments",
            "world_bank",
            "Mirny",
            "budget",
            "1.4B KZT",
            Some("2025-06-01"),
            Some("2025-12-31"),
        ),
    )
    .unwrap();

    let records = db.conflicts_for_metric("budget", &view("project:investments"));
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].cell_id, CellId(11));
    assert_eq!(records[0].source_cell_id, Some(CellId(12)));
    assert!(records[0].fact.contains("temporal conflict"));
    assert!(records[0].fact.contains("overlapping validity window"));

    db.patch_cell(
        CellId(12),
        numeric_temporal_fact_payload(
            "project:investments",
            "world_bank",
            "Mirny",
            "budget",
            "1.4B KZT",
            Some("2025-07-01"),
            Some("2025-12-31"),
        ),
    )
    .unwrap();
    assert!(db
        .conflicts_for_metric("budget", &view("project:investments"))
        .is_empty());
}

#[test]
fn citation_numeric_conflict_index_tracks_same_source_disagreement() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_cell(
        CellId(21),
        numeric_source_ref_fact_payload(
            "project:investments",
            "ifc:solar-budget",
            "report-q1.pdf",
            3,
            "Solar Plant",
            "budget",
            "1.2B KZT",
        ),
    )
    .unwrap();
    db.put_cell(
        CellId(22),
        numeric_source_ref_fact_payload(
            "project:investments",
            "ifc:solar-budget",
            "report-q1.pdf",
            3,
            "Solar Plant",
            "budget",
            "1.4B KZT",
        ),
    )
    .unwrap();

    let records = db.conflicts_for_metric("budget", &view("project:investments"));
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].cell_id, CellId(21));
    assert_eq!(records[0].source_cell_id, Some(CellId(22)));
    assert!(records[0].fact.contains("citation conflict"));
    assert!(records[0].fact.contains("same source_ref"));
}

#[test]
fn same_source_equal_value_is_not_citation_conflict() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_cell(
        CellId(31),
        numeric_source_ref_fact_payload(
            "project:investments",
            "ifc:solar-budget",
            "report-q1.pdf",
            3,
            "Solar Plant",
            "budget",
            "1.2B KZT",
        ),
    )
    .unwrap();
    db.put_cell(
        CellId(32),
        numeric_source_ref_fact_payload(
            "project:investments",
            "ifc:solar-budget",
            "report-q1.pdf",
            3,
            "Solar Plant",
            "budget",
            "1.2B KZT",
        ),
    )
    .unwrap();

    assert!(db
        .conflicts_for_metric("budget", &view("project:investments"))
        .is_empty());
}

fn numeric_fact_payload(
    scope: &str,
    source: &str,
    project: &str,
    metric: &str,
    value: &str,
) -> Vec<u8> {
    format!(
        "scope={scope}\nstatus=verified\ntype=fact\nsource={source}\nsource_trust_q16=50000\n\nproject={project}\nmetric={metric}\nvalue={value}\n{project} {metric} is {value}"
    )
    .into_bytes()
}

fn numeric_source_ref_fact_payload(
    scope: &str,
    source_id: &str,
    document_id: &str,
    page: u32,
    project: &str,
    metric: &str,
    value: &str,
) -> Vec<u8> {
    format!(
        "scope={scope}\nstatus=verified\ntype=fact\nsource_id={source_id}\ndocument_id={document_id}\npage={page}\nsource_trust_q16=50000\n\nproject={project}\nmetric={metric}\nvalue={value}\n{project} {metric} is {value}"
    )
    .into_bytes()
}

fn numeric_temporal_fact_payload(
    scope: &str,
    source: &str,
    project: &str,
    metric: &str,
    value: &str,
    valid_from: Option<&str>,
    valid_to: Option<&str>,
) -> Vec<u8> {
    let mut payload = format!(
        "scope={scope}\nstatus=verified\ntype=fact\nsource={source}\nsource_trust_q16=50000\n"
    );
    if let Some(valid_from) = valid_from {
        payload.push_str(&format!("valid_from={valid_from}\n"));
    }
    if let Some(valid_to) = valid_to {
        payload.push_str(&format!("valid_to={valid_to}\n"));
    }
    payload.push_str(&format!(
        "\nproject={project}\nmetric={metric}\nvalue={value}\n{project} {metric} is {value}"
    ));
    payload.into_bytes()
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
