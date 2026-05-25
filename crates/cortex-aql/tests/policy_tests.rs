use std::collections::BTreeSet;

use cortex_aql::{
    AgentId, AgentView, BrainId, MemoryType, PolicyError, PolicySeverity, PolicyValidator,
    RetrievalMode, ScopeId, Q16_ZERO,
};

fn view() -> AgentView {
    AgentView {
        agent_id: AgentId(1),
        readable_brains: BTreeSet::from([BrainId(10)]),
        readable_scopes: BTreeSet::from([ScopeId(20)]),
        writable_scopes: BTreeSet::new(),
        allowed_modes: BTreeSet::from([RetrievalMode::Fast, RetrievalMode::Balanced]),
        allowed_memory_types: BTreeSet::from([MemoryType::Decision]),
        max_context_budget_tokens: 1_000,
        default_context_budget_tokens: 500,
        max_candidate_limit: 50,
        default_candidate_limit: 10,
        min_required_confidence_q16: Q16_ZERO,
        max_ttl_seconds: 3_600,
        allow_remember: true,
        allow_verify_fact: false,
        allow_audit_mode: false,
        require_citations_by_default: false,
        private_scope: ScopeId(99),
    }
}

#[test]
fn budget_clamp() {
    let view = view();
    let (report, effective) = PolicyValidator::new(&view).diagnose_retrieve(
        BrainId(10),
        RetrievalMode::Balanced,
        12_000,
        10,
    );
    assert_eq!(effective.context_budget_tokens, 1_000);
    assert_eq!(
        report.diagnostics[0].code,
        PolicyError::BudgetTooHigh.code()
    );
    assert_eq!(report.diagnostics[0].severity, PolicySeverity::Clamp);
}

#[test]
fn candidate_limit_clamp() {
    let view = view();
    let (report, effective) =
        PolicyValidator::new(&view).diagnose_retrieve(BrainId(10), RetrievalMode::Fast, 500, 500);
    assert_eq!(effective.candidate_limit, 50);
    assert_eq!(
        report.diagnostics[0].code,
        PolicyError::CandidateLimitTooHigh.code()
    );
}

#[test]
fn audit_denied_without_allow_audit_mode() {
    let mut view = view();
    view.allowed_modes.insert(RetrievalMode::Audit);
    let error = PolicyValidator::new(&view)
        .enforce_retrieve(BrainId(10), RetrievalMode::Audit, 500, 10)
        .unwrap_err();
    assert_eq!(error, PolicyError::AuditModeNotAllowed);
}

#[test]
fn verify_denied_returns_verify_fact_not_allowed() {
    let view = view();
    let error = PolicyValidator::new(&view)
        .enforce_verify_fact(BrainId(10))
        .unwrap_err();
    assert_eq!(error, PolicyError::VerifyFactNotAllowed);
}

#[test]
fn remember_denied_without_write_scope() {
    let view = view();
    let error = PolicyValidator::new(&view)
        .enforce_remember(ScopeId(20), MemoryType::Decision, None)
        .unwrap_err();
    assert_eq!(error, PolicyError::ScopeNotWritable);
}
