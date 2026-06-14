use std::collections::BTreeSet;

use cortex_aql::{AgentId, AgentView, BrainId, MemoryType, RetrievalMode, Q16_ZERO};
use cortex_core::CellId;
use cortex_engine::{scope_id, Database};

#[test]
fn explain_verify_reports_logical_policy_plan_without_execution() {
    let dir = tempfile::tempdir().unwrap();
    let db = Database::open(dir.path()).unwrap();
    let report = db
        .explain_verify_aql(
            r#"EXPLAIN VERIFY FACT "Solar budget is 12 KZT" IN BRAIN default;"#,
            &view(),
        )
        .unwrap();

    assert_eq!(report.fact, "Solar budget is 12 KZT");
    assert_eq!(report.max_candidates, 20);
    assert_eq!(report.max_evidence, 8);
    assert!(report.execution_trace.is_none());
    assert!(!report.logical_plan.policy_complete);
    assert!(report.policy_rewritten_plan.policy_complete);
    assert!(report
        .logical_plan
        .nodes
        .iter()
        .any(|node| node.kind == "verify"));
}

#[test]
fn explain_analyze_verify_reports_operator_trace_and_counts() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_cell(
        CellId(1),
        b"scope=tenant:private\nmetric=budget\nvalue=12 KZT\ncitation=doc-a\n\nSolar budget is 12 KZT."
            .to_vec(),
    )
    .unwrap();

    let report = db
        .explain_analyze_verify_aql(
            r#"EXPLAIN ANALYZE VERIFY FACT "Solar budget is 12 KZT" IN BRAIN default;"#,
            &view(),
        )
        .unwrap();
    let trace = report.execution_trace.expect("analyze returns trace");

    assert!(report.status.is_some());
    assert!(report.confidence_q16.is_some());
    assert!(report.evidence_count >= 1);
    assert!(trace
        .operators
        .iter()
        .any(|operator| operator.name == "VerificationCandidateScan"));
    assert!(trace
        .operators
        .iter()
        .any(|operator| operator.name == "VerificationPermissionFilter"));
    assert!(trace
        .operators
        .iter()
        .any(|operator| operator.name == "VerifyOp" && operator.output_count >= 1));
    assert!(trace
        .operators
        .iter()
        .any(|operator| operator.name == "VerdictAggregateOp" && operator.output_count == 1));
}

#[test]
fn explain_analyze_verify_uses_temporal_index_for_stale_guard() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_cell(
        CellId(11),
        b"scope=tenant:private\nstatus=verified\ntype=fact\nsource=archive\nvalid_to=2024-12-31\n\nSolar budget is 12 KZT."
            .to_vec(),
    )
    .unwrap();

    let report = db
        .explain_analyze_verify_aql(
            r#"EXPLAIN ANALYZE VERIFY FACT "Solar budget is 12 KZT on 2025-01-10" IN BRAIN default;"#,
            &view(),
        )
        .unwrap();
    let trace = report.execution_trace.expect("analyze returns trace");

    assert_eq!(
        report.status,
        Some(cortex_engine::VerificationStatus::Insufficient)
    );
    assert!(trace.operators.iter().any(|operator| {
        operator.name == "VerificationTemporalIndexLookup" && operator.output_count == 1
    }));
    assert_eq!(report.guard_count, 1);
}

fn view() -> AgentView {
    AgentView {
        agent_id: AgentId(1),
        label: None,
        readable_brains: BTreeSet::from([BrainId(1)]),
        readable_scopes: BTreeSet::from([scope_id("tenant:private")]),
        writable_scopes: BTreeSet::new(),
        allowed_modes: BTreeSet::from([RetrievalMode::Balanced]),
        allowed_memory_types: BTreeSet::from([MemoryType::Decision]),
        max_context_budget_tokens: 1_000,
        default_context_budget_tokens: 400,
        max_candidate_limit: 100,
        default_candidate_limit: 20,
        min_required_confidence_q16: Q16_ZERO,
        max_ttl_seconds: None,
        allow_remember: false,
        allow_verify_fact: true,
        allow_audit_mode: false,
        require_citations_by_default: false,
        private_scope: None,
    }
}
