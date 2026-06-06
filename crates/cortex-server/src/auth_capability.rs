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
    pub tenants: Option<BTreeSet<String>>,
    pub request_quota_per_minute: Option<u64>,
    pub body_quota_bytes_per_minute: Option<u64>,
    pub queue_quota: Option<u64>,
    pub context_budget_tokens: Option<u32>,
    pub capabilities: Option<BTreeSet<AuthCapability>>,
}

impl EffectiveAuthPolicy {
    pub(crate) fn from_token_policy(policy: AuthTokenPolicy) -> Self {
        Self {
            token: policy.token,
            role: policy.role,
            agent_id: policy.agent_id,
            principal_id: policy.principal_id,
            tenants: None,
            request_quota_per_minute: policy.request_quota_per_minute,
            body_quota_bytes_per_minute: policy.body_quota_bytes_per_minute,
            queue_quota: policy.queue_quota,
            context_budget_tokens: policy.context_budget_tokens,
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

    pub(crate) fn with_tenants(mut self, tenants: BTreeSet<String>) -> Self {
        self.tenants = Some(tenants);
        self
    }

    pub(crate) fn with_body_quota_bytes_per_minute(mut self, quota: u64) -> Self {
        self.body_quota_bytes_per_minute = Some(quota);
        self
    }

    pub(crate) fn with_queue_quota(mut self, quota: u64) -> Self {
        self.queue_quota = Some(quota);
        self
    }

    pub(crate) fn with_context_budget_tokens(mut self, budget: u32) -> Self {
        self.context_budget_tokens = Some(budget);
        self
    }

    pub(crate) fn with_capabilities(mut self, capabilities: BTreeSet<AuthCapability>) -> Self {
        self.capabilities = Some(capabilities);
        self
    }

    pub(crate) fn quota_key(&self) -> String {
        self.principal_id
            .clone()
            .unwrap_or_else(|| token_fingerprint(&self.token))
    }
}

fn token_fingerprint(token: &str) -> String {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for byte in token.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("token:{hash:016x}")
}
