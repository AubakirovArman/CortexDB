use crate::responses::RouterError;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LlmInferenceAuditOutcome {
    Allowed,
    Denied,
}

impl LlmInferenceAuditOutcome {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Allowed => "allowed",
            Self::Denied => "denied",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct LlmInferenceDecisionAudit {
    pub(crate) outcome: LlmInferenceAuditOutcome,
    pub(crate) reason: &'static str,
    pub(crate) provider: &'static str,
    pub(crate) model: &'static str,
    pub(crate) context_cell_count: u64,
    pub(crate) citation_count: u64,
    pub(crate) request_api_key_present: bool,
}

impl LlmInferenceDecisionAudit {
    pub(crate) fn allowed(context_cell_count: u64, citation_count: u64) -> Self {
        Self {
            outcome: LlmInferenceAuditOutcome::Allowed,
            reason: "test_double_completed",
            provider: "test_double",
            model: "deterministic-echo-v1",
            context_cell_count,
            citation_count,
            request_api_key_present: false,
        }
    }

    pub(crate) fn denied(reason: &'static str) -> Self {
        Self {
            outcome: LlmInferenceAuditOutcome::Denied,
            reason,
            provider: "test_double",
            model: "deterministic-echo-v1",
            context_cell_count: 0,
            citation_count: 0,
            request_api_key_present: false,
        }
    }

    pub(crate) fn denied_for_request(
        reason: &'static str,
        context_cell_count: u64,
        citation_count: u64,
        request_api_key_present: bool,
    ) -> Self {
        Self {
            outcome: LlmInferenceAuditOutcome::Denied,
            reason,
            provider: "test_double",
            model: "deterministic-echo-v1",
            context_cell_count,
            citation_count,
            request_api_key_present,
        }
    }
}

#[derive(Debug)]
pub(crate) struct LlmInferenceResult {
    pub(crate) body: String,
    pub(crate) audit: LlmInferenceDecisionAudit,
}

#[derive(Debug)]
pub(crate) struct LlmInferenceRejection {
    pub(crate) error: RouterError,
    pub(crate) audit: LlmInferenceDecisionAudit,
}

impl LlmInferenceRejection {
    pub(crate) fn new(error: RouterError, audit: LlmInferenceDecisionAudit) -> Self {
        Self { error, audit }
    }
}
