use std::collections::BTreeSet;

use crate::audit::AuditAction;
use crate::auth::{AuthRole, AuthTokenPolicy};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum AuthCapability {
    Admin,
    Aql,
    Context,
    Delete,
    Ingest,
    Inference,
    Memory,
    Metrics,
    Read,
    Search,
    Verify,
    Write,
}

impl AuthCapability {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Admin => "admin",
            Self::Aql => "aql",
            Self::Context => "context",
            Self::Delete => "delete",
            Self::Ingest => "ingest",
            Self::Inference => "inference",
            Self::Memory => "memory",
            Self::Metrics => "metrics",
            Self::Read => "read",
            Self::Search => "search",
            Self::Verify => "verify",
            Self::Write => "write",
        }
    }

    pub(crate) fn parse(raw: &str) -> Result<Self, String> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "admin" => Ok(Self::Admin),
            "aql" => Ok(Self::Aql),
            "context" => Ok(Self::Context),
            "delete" => Ok(Self::Delete),
            "ingest" => Ok(Self::Ingest),
            "inference" => Ok(Self::Inference),
            "memory" => Ok(Self::Memory),
            "metrics" => Ok(Self::Metrics),
            "read" => Ok(Self::Read),
            "search" => Ok(Self::Search),
            "verify" => Ok(Self::Verify),
            "write" => Ok(Self::Write),
            _ => Err("auth capability is not recognized".to_owned()),
        }
    }

    pub(crate) fn allows(self, action: AuditAction) -> bool {
        matches!(
            (self, action),
            (Self::Admin, AuditAction::Admin)
                | (Self::Aql, AuditAction::Aql)
                | (Self::Context, AuditAction::Context)
                | (Self::Delete, AuditAction::Delete)
                | (Self::Ingest, AuditAction::Ingest)
                | (Self::Inference, AuditAction::Inference)
                | (Self::Memory, AuditAction::Memory)
                | (Self::Metrics, AuditAction::Metrics)
                | (Self::Read, AuditAction::Read)
                | (Self::Search, AuditAction::Search)
                | (Self::Verify, AuditAction::Verify)
                | (Self::Write, AuditAction::Write)
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct EffectiveAuthPolicy {
    pub token: String,
    pub role: AuthRole,
    pub agent_id: Option<u64>,
    pub principal_id: Option<String>,
    pub request_quota_per_minute: Option<u64>,
    pub capabilities: Option<BTreeSet<AuthCapability>>,
}

impl EffectiveAuthPolicy {
    pub(crate) fn from_token_policy(policy: AuthTokenPolicy) -> Self {
        Self {
            token: policy.token,
            role: policy.role,
            agent_id: policy.agent_id,
            principal_id: policy.principal_id,
            request_quota_per_minute: policy.request_quota_per_minute,
            capabilities: None,
        }
    }

    pub(crate) fn with_agent_id(mut self, agent_id: u64) -> Self {
        self.agent_id = Some(agent_id);
        self
    }

    pub(crate) fn with_request_quota_per_minute(mut self, quota: u64) -> Self {
        self.request_quota_per_minute = Some(quota);
        self
    }

    pub(crate) fn with_capabilities(mut self, capabilities: BTreeSet<AuthCapability>) -> Self {
        self.capabilities = Some(capabilities);
        self
    }
}
