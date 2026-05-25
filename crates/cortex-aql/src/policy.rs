use crate::agent_view::AgentView;
use crate::types::{BrainId, MemoryType, RetrievalMode, ScopeId, Q16};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PolicyError {
    BrainNotReadable,
    ScopeNotReadable,
    ScopeNotWritable,
    ModeNotAllowed,
    AuditModeNotAllowed,
    RememberNotAllowed,
    VerifyFactNotAllowed,
    MemoryTypeNotAllowed,
    CandidateLimitTooHigh,
    BudgetTooHigh,
    TtlTooLong,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PolicySeverity {
    Deny,
    Clamp,
    Warn,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PolicyCode {
    BrainNotReadable,
    ScopeNotReadable,
    ScopeNotWritable,
    ModeNotAllowed,
    AuditModeNotAllowed,
    RememberNotAllowed,
    VerifyFactNotAllowed,
    MemoryTypeNotAllowed,
    CandidateLimitTooHigh,
    BudgetTooHigh,
    TtlTooLong,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PolicyDiagnostic {
    pub code: PolicyCode,
    pub severity: PolicySeverity,
    pub safe_message: &'static str,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PolicyDiagnosticExport {
    pub code: PolicyCode,
    pub severity: PolicySeverity,
    pub message: &'static str,
    pub agent_id: Option<crate::types::AgentId>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PolicyReport {
    pub diagnostics: Vec<PolicyDiagnostic>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EffectiveRetrievePolicy {
    pub mode: RetrievalMode,
    pub context_budget_tokens: u32,
    pub candidate_limit: u32,
    pub min_required_confidence_q16: Q16,
    pub require_citations: bool,
}

pub struct PolicyValidator<'a> {
    view: &'a AgentView,
}

impl<'a> PolicyValidator<'a> {
    pub fn new(view: &'a AgentView) -> Self {
        Self { view }
    }

    pub fn enforce_retrieve(
        &self,
        brain: BrainId,
        mode: RetrievalMode,
        requested_budget_tokens: u32,
        requested_candidate_limit: u32,
    ) -> Result<EffectiveRetrievePolicy, PolicyError> {
        if !self.view.can_read_brain(brain) {
            return Err(PolicyError::BrainNotReadable);
        }
        if !self.view.can_use_mode(mode) {
            return Err(PolicyError::ModeNotAllowed);
        }
        if mode == RetrievalMode::Audit && !self.view.can_use_audit_mode() {
            return Err(PolicyError::AuditModeNotAllowed);
        }
        Ok(self.effective_retrieve(mode, requested_budget_tokens, requested_candidate_limit))
    }

    pub fn diagnose_retrieve(
        &self,
        brain: BrainId,
        mode: RetrievalMode,
        requested_budget_tokens: u32,
        requested_candidate_limit: u32,
    ) -> (PolicyReport, EffectiveRetrievePolicy) {
        let mut report = PolicyReport::default();
        if !self.view.can_read_brain(brain) {
            report.push(PolicyError::BrainNotReadable);
        }
        if !self.view.can_use_mode(mode) {
            report.push(PolicyError::ModeNotAllowed);
        }
        if mode == RetrievalMode::Audit && !self.view.can_use_audit_mode() {
            report.push(PolicyError::AuditModeNotAllowed);
        }
        if requested_budget_tokens > self.view.max_context_budget_tokens {
            report.push(PolicyError::BudgetTooHigh);
        }
        if requested_candidate_limit > self.view.max_candidate_limit {
            report.push(PolicyError::CandidateLimitTooHigh);
        }
        (
            report,
            self.effective_retrieve(mode, requested_budget_tokens, requested_candidate_limit),
        )
    }

    pub fn enforce_remember(
        &self,
        scope: ScopeId,
        memory_type: MemoryType,
        ttl_seconds: Option<u64>,
    ) -> Result<(), PolicyError> {
        if !self.view.allow_remember {
            return Err(PolicyError::RememberNotAllowed);
        }
        if !self.view.can_write_scope(scope) {
            return Err(PolicyError::ScopeNotWritable);
        }
        if !self.view.can_remember_type(memory_type) {
            return Err(PolicyError::MemoryTypeNotAllowed);
        }
        if let (Some(ttl), Some(max_ttl)) = (ttl_seconds, self.view.max_ttl_seconds) {
            if ttl > max_ttl {
                return Err(PolicyError::TtlTooLong);
            }
        }
        Ok(())
    }

    pub fn enforce_verify_fact(&self, brain: BrainId) -> Result<(), PolicyError> {
        if !self.view.can_verify_fact() {
            return Err(PolicyError::VerifyFactNotAllowed);
        }
        if !self.view.can_read_brain(brain) {
            return Err(PolicyError::BrainNotReadable);
        }
        Ok(())
    }

    fn effective_retrieve(
        &self,
        mode: RetrievalMode,
        requested_budget_tokens: u32,
        requested_candidate_limit: u32,
    ) -> EffectiveRetrievePolicy {
        EffectiveRetrievePolicy {
            mode,
            context_budget_tokens: self.view.effective_budget(requested_budget_tokens),
            candidate_limit: self
                .view
                .effective_candidate_limit(requested_candidate_limit),
            min_required_confidence_q16: self.view.min_required_confidence_q16,
            require_citations: self.view.require_citations_by_default,
        }
    }
}

impl PolicyReport {
    pub fn has_denies(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity == PolicySeverity::Deny)
    }

    pub fn allowed(&self) -> bool {
        !self.has_denies()
    }

    pub fn has_clamps(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity == PolicySeverity::Clamp)
    }

    pub fn safe_export(&self) -> Vec<PolicyDiagnosticExport> {
        self.diagnostics
            .iter()
            .map(|diagnostic| PolicyDiagnosticExport {
                code: diagnostic.code,
                severity: diagnostic.severity,
                message: diagnostic.safe_message,
                agent_id: None,
            })
            .collect()
    }

    pub fn internal_export(&self, agent_id: crate::types::AgentId) -> Vec<PolicyDiagnosticExport> {
        self.diagnostics
            .iter()
            .map(|diagnostic| PolicyDiagnosticExport {
                code: diagnostic.code,
                severity: diagnostic.severity,
                message: diagnostic.safe_message,
                agent_id: Some(agent_id),
            })
            .collect()
    }

    fn push(&mut self, error: PolicyError) {
        self.diagnostics.push(error.into());
    }
}

impl From<PolicyError> for PolicyDiagnostic {
    fn from(error: PolicyError) -> Self {
        Self {
            code: error.code(),
            severity: error.severity(),
            safe_message: error.safe_message(),
        }
    }
}

impl PolicyError {
    pub fn code(self) -> PolicyCode {
        match self {
            Self::BrainNotReadable => PolicyCode::BrainNotReadable,
            Self::ScopeNotReadable => PolicyCode::ScopeNotReadable,
            Self::ScopeNotWritable => PolicyCode::ScopeNotWritable,
            Self::ModeNotAllowed => PolicyCode::ModeNotAllowed,
            Self::AuditModeNotAllowed => PolicyCode::AuditModeNotAllowed,
            Self::RememberNotAllowed => PolicyCode::RememberNotAllowed,
            Self::VerifyFactNotAllowed => PolicyCode::VerifyFactNotAllowed,
            Self::MemoryTypeNotAllowed => PolicyCode::MemoryTypeNotAllowed,
            Self::CandidateLimitTooHigh => PolicyCode::CandidateLimitTooHigh,
            Self::BudgetTooHigh => PolicyCode::BudgetTooHigh,
            Self::TtlTooLong => PolicyCode::TtlTooLong,
        }
    }

    pub fn severity(self) -> PolicySeverity {
        match self {
            Self::BudgetTooHigh | Self::CandidateLimitTooHigh => PolicySeverity::Clamp,
            _ => PolicySeverity::Deny,
        }
    }

    pub fn safe_message(self) -> &'static str {
        match self {
            Self::BrainNotReadable => "requested brain is not readable",
            Self::ScopeNotReadable => "requested scope is not readable",
            Self::ScopeNotWritable => "requested scope is not writable",
            Self::ModeNotAllowed => "requested retrieval mode is not allowed",
            Self::AuditModeNotAllowed => "audit mode is not allowed",
            Self::RememberNotAllowed => "remember operation is not allowed",
            Self::VerifyFactNotAllowed => "verify operation is not allowed",
            Self::MemoryTypeNotAllowed => "memory type is not allowed",
            Self::CandidateLimitTooHigh => "candidate limit was clamped",
            Self::BudgetTooHigh => "context budget was clamped",
            Self::TtlTooLong => "ttl is longer than allowed",
        }
    }
}
