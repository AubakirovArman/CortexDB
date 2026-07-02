use std::collections::BTreeSet;

use cortex_aql::{AgentId, AgentView, BrainId, MemoryType, RetrievalMode, Q16_ZERO};
use cortex_core::CellId;
use cortex_engine::{
    scope_id, AnnFallbackReason, AnnSearchPolicy, ContextPackAnomalyCode, ContextPackExportFormat,
    ContextPackOptions, Database, DatabaseOptions, EngineFeatureFlags, SearchLimit,
};

#[test]
fn ann_visit_budget_exhaustion_is_disclosed_in_context_pack_exports() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open_with_options(dir.path(), hnsw_options()).unwrap();
    write_vector_cells(&mut db, 16);
    db.checkpoint().unwrap();

    let view = view("project:investments");
    let outcome = db
        .search_vector_with_report_with_policy(
            &[16, 0],
            &view,
            SearchLimit(4),
            AnnSearchPolicy {
                max_visited_candidates: Some(0),
                fallback: true,
                ..AnnSearchPolicy::default()
            },
        )
        .unwrap();

    let report = outcome.ann_report.as_ref().expect("vector search report");
    assert_eq!(
        report.fallback_reason,
        Some(AnnFallbackReason::VisitBudgetExceeded)
    );
    assert!(report.fallback_performed);
    assert!(!outcome.results.is_empty());

    let pack = db.context_pack_from_search_outcome_with_options(
        outcome,
        &view,
        1_000,
        false,
        &ContextPackOptions::default(),
        "vector budget",
    );

    assert!(pack.anomalies.iter().any(|anomaly| {
        anomaly.cell_id.is_none() && anomaly.code == ContextPackAnomalyCode::RetrievalIncomplete
    }));

    for exported in [
        pack.export(ContextPackExportFormat::Json),
        pack.export(ContextPackExportFormat::Prompt),
        pack.export(ContextPackExportFormat::Markdown),
    ] {
        assert!(exported.contains("retrieval_incomplete"));
    }
}

#[test]
fn complete_ann_search_does_not_report_retrieval_incomplete() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open_with_options(dir.path(), hnsw_options()).unwrap();
    write_vector_cells(&mut db, 8);
    db.checkpoint().unwrap();

    let view = view("project:investments");
    let outcome = db
        .search_vector_with_report_with_policy(
            &[8, 0],
            &view,
            SearchLimit(3),
            AnnSearchPolicy::default(),
        )
        .unwrap();
    let pack = db.context_pack_from_search_outcome_with_options(
        outcome,
        &view,
        1_000,
        false,
        &ContextPackOptions::default(),
        "vector budget",
    );

    assert!(pack
        .anomalies
        .iter()
        .all(|anomaly| anomaly.code != ContextPackAnomalyCode::RetrievalIncomplete));
}

fn write_vector_cells(db: &mut Database, count: u64) {
    for id in 1..=count {
        db.put_cell(
            CellId(id),
            format!(
                "scope=project:investments\nstatus=ready\nvector={},{}\n\ncell {id} budget evidence",
                id,
                count - id
            )
            .into_bytes(),
        )
        .unwrap();
    }
}

fn hnsw_options() -> DatabaseOptions {
    DatabaseOptions {
        feature_flags: EngineFeatureFlags::production_safe().with_experimental_hnsw(true),
        ..DatabaseOptions::default()
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
        allow_verify_fact: false,
        allow_audit_mode: false,
        require_citations_by_default: false,
        private_scope: None,
    }
}
