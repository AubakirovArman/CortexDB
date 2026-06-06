use std::collections::BTreeSet;

use cortex_aql::{AgentId, AgentView, BrainId, MemoryType, RetrievalMode, Q16_ZERO};
use cortex_core::CellId;
use cortex_engine::{scope_id, ContextPackAnomalyCode, ContextPackOptions, Database};

#[test]
fn require_citations_from_aql_reaches_context_pack() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_cell(
        CellId(1),
        b"scope=project:investments\nstatus=ready\nalpha budget".to_vec(),
    )
    .unwrap();

    let pack = db
        .context_pack_from_aql(
            r#"RETRIEVE CONTEXT FOR TASK "budget" IN BRAIN investment_projects
WHERE space = project:investments AND status = "ready" LIMIT 10 CANDIDATES
REQUIRE citations;"#,
            &view(),
            ContextPackOptions::default(),
        )
        .unwrap();

    assert!(pack.citations_required);
    assert_eq!(pack.anomalies.len(), 1);
    assert_eq!(
        pack.anomalies[0].code,
        ContextPackAnomalyCode::MissingCitation
    );
    assert_eq!(pack.anomalies[0].cell_id, Some(CellId(1)));
}

#[test]
fn require_confidence_filters_retrieval_candidates() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_cell(
        CellId(1),
        b"scope=project:investments\nstatus=ready\nsource_id=ifc:high\nconfidence_q16=60000\nalpha budget".to_vec(),
    )
    .unwrap();
    db.put_cell(
        CellId(2),
        b"scope=project:investments\nstatus=ready\nsource_id=ifc:low\nconfidence_q16=30000\nalpha budget".to_vec(),
    )
    .unwrap();

    let pack = db
        .context_pack_from_aql(
            r#"RETRIEVE CONTEXT FOR TASK "budget" IN BRAIN investment_projects
WHERE space = project:investments AND status = "ready" LIMIT 10 CANDIDATES
REQUIRE confidence >= 0.80;"#,
            &view(),
            ContextPackOptions::default(),
        )
        .unwrap();

    assert_cell_ids(&pack, &[CellId(1)]);
}

#[test]
fn require_source_trust_filters_retrieval_candidates() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_cell(
        CellId(1),
        b"scope=project:investments\nstatus=ready\nsource=official\nsource_trust_q16=60000\nalpha budget".to_vec(),
    )
    .unwrap();
    db.put_cell(
        CellId(2),
        b"scope=project:investments\nstatus=ready\nsource=unreviewed\nsource_trust_q16=30000\nalpha budget".to_vec(),
    )
    .unwrap();

    let pack = db
        .context_pack_from_aql(
            r#"RETRIEVE CONTEXT FOR TASK "budget" IN BRAIN investment_projects
WHERE space = project:investments AND status = "ready" LIMIT 10 CANDIDATES
REQUIRE source_trust >= 0.80;"#,
            &view(),
            ContextPackOptions::default(),
        )
        .unwrap();

    assert_cell_ids(&pack, &[CellId(1)]);
    assert!(!pack.citations_required);
}

#[test]
fn require_freshness_filters_stale_candidates() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    let now = unix_now_seconds();
    let fresh = format!(
        "scope=project:investments\nstatus=ready\ncreated_unix_seconds={}\nalpha budget",
        now.saturating_sub(30)
    );
    let stale = format!(
        "scope=project:investments\nstatus=ready\ncreated_unix_seconds={}\nalpha budget",
        now.saturating_sub(120)
    );
    db.put_cell(CellId(1), fresh.into_bytes()).unwrap();
    db.put_cell(CellId(2), stale.into_bytes()).unwrap();

    let pack = db
        .context_pack_from_aql(
            r#"RETRIEVE CONTEXT FOR TASK "budget" IN BRAIN investment_projects
WHERE space = project:investments AND status = "ready" LIMIT 10 CANDIDATES
REQUIRE freshness <= 60 SECONDS;"#,
            &view(),
            ContextPackOptions::default(),
        )
        .unwrap();

    assert_cell_ids(&pack, &[CellId(1)]);
}

fn assert_cell_ids(pack: &cortex_engine::ContextPack, expected: &[CellId]) {
    let actual = pack
        .cells
        .iter()
        .map(|cell| cell.cell_id)
        .collect::<Vec<_>>();
    assert_eq!(actual, expected);
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

fn unix_now_seconds() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}
