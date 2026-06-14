use std::collections::BTreeSet;

use cortex_aql::{AgentId, AgentView, BrainId, MemoryType, RetrievalMode, Q16_ZERO};
use cortex_core::CellId;
use cortex_engine::{scope_id, ContextPackOptions, Database, DatabaseOptions, PayloadResidency};

#[test]
fn require_valid_at_filters_temporal_candidates_before_payload_reads() {
    let dir = tempfile::tempdir().unwrap();
    {
        let mut db = Database::open(dir.path()).unwrap();
        db.put_cell(
            CellId(1),
            b"scope=project:investments\nstatus=ready\nvalid_from=2025-01-01\nvalid_to=2025-12-31\n\nalpha budget current".to_vec(),
        )
        .unwrap();
        db.put_cell(
            CellId(2),
            b"scope=project:investments\nstatus=ready\nvalid_to=2024-12-31\n\nalpha budget expired"
                .to_vec(),
        )
        .unwrap();
        db.put_cell(
            CellId(3),
            b"scope=project:investments\nstatus=ready\nvalid_from=2026-01-01\n\nalpha budget future".to_vec(),
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
    let pack = db
        .context_pack_from_aql(
            r#"RETRIEVE CONTEXT FOR TASK "budget" IN BRAIN investment_projects
WHERE space = project:investments AND status = "ready" LIMIT 10 CANDIDATES
REQUIRE valid at "2025-06-01";"#,
            &view(),
            ContextPackOptions::default(),
        )
        .unwrap();

    assert_eq!(
        pack.cells
            .iter()
            .map(|cell| cell.cell_id)
            .collect::<Vec<_>>(),
        [CellId(1)]
    );
    assert_eq!(
        db.payload_cache_stats().segment_loads, 1,
        "temporal validity must filter expired/future candidates before lazy payload materialization"
    );
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
        allow_verify_fact: false,
        allow_audit_mode: false,
        require_citations_by_default: false,
        private_scope: None,
    }
}
