use std::collections::BTreeSet;

use cortex_aql::{AgentId, AgentView, BrainId, MemoryType, RetrievalMode, Q16_ZERO};
use cortex_core::CellId;
use cortex_engine::verification::{VerificationGuardCode, VerificationStatus};
use cortex_engine::{scope_id, ContextPackAnomalyCode, ContextPackOptions, Database};

#[test]
fn context_and_verify_quality_fixture_is_stable_before_checkpoint() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    seed_quality_fixture(&mut db);

    assert_context_quality(&db);
    assert_verify_quality(&db);
}

#[test]
fn context_and_verify_quality_fixture_survives_checkpoint_restart() {
    let dir = tempfile::tempdir().unwrap();
    {
        let mut db = Database::open(dir.path()).unwrap();
        seed_quality_fixture(&mut db);
        db.checkpoint().unwrap();
    }

    let db = Database::open(dir.path()).unwrap();
    assert_context_quality(&db);
    assert_verify_quality(&db);
}

fn seed_quality_fixture(db: &mut Database) {
    db.put_cell(
        CellId(1),
        br#"scope=project:investments
status=ready
type=fact
source=report-q1.pdf#page=3
source_trust=0.95
project=Solar Plant
metric=budget
value=1200000000
currency=KZT

Solar Plant budget is 1.2B KZT for 2025."#
            .to_vec(),
    )
    .unwrap();
    db.put_cell(
        CellId(2),
        br#"scope=project:investments
status=ready
type=fact
source=report-q2.pdf#page=5
source_trust=0.90
project=Solar Plant
metric=budget
value=1400000000
currency=KZT

Solar Plant budget is 1.4B KZT for 2025."#
            .to_vec(),
    )
    .unwrap();
    db.put_cell(
        CellId(3),
        br#"scope=tenant:private
status=ready
type=fact
source=private.pdf#page=1

Solar Plant budget is 2.0B KZT for 2025."#
            .to_vec(),
    )
    .unwrap();
}

fn assert_context_quality(db: &Database) {
    let pack = db
        .context_pack_from_aql(
            context_query(),
            &agent_view(true, false),
            ContextPackOptions {
                token_budget_tokens: 1_000,
                require_citations: true,
                reduce_redundancy: true,
                redundancy_threshold_q16: 10,
            },
        )
        .unwrap();

    let ids = pack
        .cells
        .iter()
        .map(|cell| cell.cell_id)
        .collect::<Vec<_>>();
    assert_eq!(ids, vec![CellId(1), CellId(2)]);
    assert!(pack.estimated_tokens <= pack.token_budget_tokens);
    assert!(!pack.truncated);
    assert!(pack.citations_required);
    assert!(pack.anomalies.iter().all(|anomaly| {
        anomaly.code != ContextPackAnomalyCode::MissingCitation
            && anomaly.code != ContextPackAnomalyCode::RedundantCell
    }));
    assert_eq!(
        pack.cells
            .iter()
            .map(|cell| cell.citation.as_deref())
            .collect::<Vec<_>>(),
        vec![Some("report-q1.pdf#page=3"), Some("report-q2.pdf#page=5")]
    );
    assert!(pack.cells.iter().all(|cell| cell
        .explain
        .as_ref()
        .is_some_and(|explain| explain.matched_terms.iter().any(|term| term == "budget"))));
}

fn assert_verify_quality(db: &Database) {
    let report = db
        .verify_fact_aql(
            r#"VERIFY FACT "Solar Plant budget is 1.2B KZT for 2025" IN BRAIN investment_projects;"#,
            &agent_view(true, true),
        )
        .unwrap();

    assert_eq!(report.status, VerificationStatus::Mixed);
    assert_eq!(report.evidence.len(), 1);
    assert_eq!(report.evidence[0].cell_id, CellId(1));
    assert_eq!(
        report.evidence[0].citation.as_deref(),
        Some("report-q1.pdf#page=3")
    );
    assert_eq!(report.contradicting_evidence.len(), 1);
    assert_eq!(report.contradicting_evidence[0].cell_id, CellId(2));
    assert_eq!(
        report.contradicting_evidence[0].citation.as_deref(),
        Some("report-q2.pdf#page=5")
    );
    assert!(report.guards.iter().any(|guard| {
        guard.cell_id == Some(CellId(2)) && guard.code == VerificationGuardCode::NumericMismatch
    }));
    assert!(report
        .guards
        .iter()
        .all(|guard| guard.code != VerificationGuardCode::MissingCitation));
}

fn context_query() -> &'static str {
    r#"RETRIEVE CONTEXT
FOR TASK "Solar Plant budget 2025"
IN BRAIN investment_projects
WHERE space = project:investments AND status = "ready"
LIMIT 10 CANDIDATES;"#
}

fn agent_view(require_citations: bool, allow_verify: bool) -> AgentView {
    AgentView {
        agent_id: AgentId(1),
        label: Some("quality-fixture-agent".to_owned()),
        readable_brains: BTreeSet::from([BrainId(1)]),
        readable_scopes: BTreeSet::from([scope_id("project:investments")]),
        writable_scopes: BTreeSet::new(),
        allowed_modes: BTreeSet::from([RetrievalMode::Balanced]),
        allowed_memory_types: BTreeSet::from([MemoryType::Decision]),
        max_context_budget_tokens: 2_000,
        default_context_budget_tokens: 1_000,
        max_candidate_limit: 100,
        default_candidate_limit: 20,
        min_required_confidence_q16: Q16_ZERO,
        max_ttl_seconds: Some(3_600),
        allow_remember: false,
        allow_verify_fact: allow_verify,
        allow_audit_mode: false,
        require_citations_by_default: require_citations,
        private_scope: None,
    }
}
