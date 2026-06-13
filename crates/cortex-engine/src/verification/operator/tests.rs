use std::collections::BTreeSet;

use cortex_aql::{
    AgentId, AgentView, BoundVerifyFactPlan, BrainId, MemoryType, RetrievalMode, Q16_ZERO,
};
use cortex_core::CellId;

use crate::query::scope_id;
use crate::Database;

#[test]
fn verify_execution_report_traces_operator_stages_and_preserves_report() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_cell(
        CellId(20),
        b"scope=tenant:private\nmetric=budget\nvalue=12 KZT\nproject=Solar\n\nSolar budget is 12 KZT."
            .to_vec(),
    )
    .unwrap();
    let view = view("tenant:private");
    let fact = "Solar budget is 12 KZT";

    let execution = db
        .execute_verify_fact_plan(
            BoundVerifyFactPlan {
                brain_id: BrainId(1),
                fact: fact.to_owned(),
                max_candidates: 20,
                max_evidence: 8,
            },
            &view,
        )
        .unwrap();
    let public_report = db
        .verify_fact_aql(
            r#"VERIFY FACT "Solar budget is 12 KZT" IN BRAIN default;"#,
            &view,
        )
        .unwrap();

    assert_eq!(execution.report, public_report);
    assert!(execution.operators.len() >= 4);
    assert!(execution
        .report
        .evidence
        .iter()
        .any(|item| item.cell_id == CellId(20)));
    assert!(execution
        .operators
        .iter()
        .any(|trace| trace.name == "VerificationCandidateScan" && trace.output_count >= 1));
    assert!(execution
        .operators
        .iter()
        .any(|trace| trace.name == "VerifyOp" && trace.output_count >= 1));
    assert!(execution
        .operators
        .iter()
        .any(|trace| trace.name == "VerdictAggregateOp" && trace.output_count == 1));
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
        max_ttl_seconds: None,
        allow_remember: false,
        allow_verify_fact: true,
        allow_audit_mode: false,
        require_citations_by_default: false,
        private_scope: None,
    }
}
