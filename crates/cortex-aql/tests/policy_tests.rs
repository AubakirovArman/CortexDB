use std::collections::BTreeSet;

use cortex_aql::{
    AgentId, AgentView, BrainId, MemoryType, PolicyError, PolicySeverity, PolicyValidator,
    RetrievalMode, ScopeId, Q16_ZERO,
};

fn view() -> AgentView {
    AgentView {
        agent_id: AgentId(1),
        label: None,
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
        max_ttl_seconds: Some(3_600),
        allow_remember: true,
        allow_verify_fact: false,
        allow_audit_mode: false,
        require_citations_by_default: false,
        private_scope: Some(ScopeId(99)),
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
fn report_allowed_and_clamps_are_exposed() {
    let view = view();
    let (report, _) =
        PolicyValidator::new(&view).diagnose_retrieve(BrainId(10), RetrievalMode::Fast, 500, 500);
    assert!(report.allowed());
    assert!(report.has_clamps());
    assert!(report.safe_export()[0].agent_id.is_none());
    assert_eq!(
        report.internal_export(AgentId(1))[0].agent_id,
        Some(AgentId(1))
    );
}

#[test]
fn agent_view_helpers_respect_capabilities() {
    let mut view = view();
    assert!(!view.can_use_audit_mode());
    assert!(!view.can_verify_fact());
    assert!(view.can_remember_type(MemoryType::Decision));
    view.allow_remember = false;
    assert!(!view.can_remember_type(MemoryType::Decision));
    assert_eq!(view.effective_budget(10_000), 1_000);
    assert_eq!(view.effective_candidate_limit(500), 50);
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
