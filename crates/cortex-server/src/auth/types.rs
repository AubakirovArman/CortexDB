#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuthRole {
    Admin,
    Data,
}

impl AuthRole {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Admin => "admin",
            Self::Data => "data",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuthTokenPolicy {
    pub token: String,
    pub role: AuthRole,
    pub agent_id: Option<u64>,
    pub principal_id: Option<String>,
    pub request_quota_per_minute: Option<u64>,
    pub body_quota_bytes_per_minute: Option<u64>,
    pub queue_quota: Option<u64>,
    pub context_budget_tokens: Option<u32>,
}

impl AuthTokenPolicy {
    pub fn new(token: impl Into<String>, role: AuthRole) -> Self {
        Self {
            token: token.into(),
            role,
            agent_id: None,
            principal_id: None,
            request_quota_per_minute: None,
            body_quota_bytes_per_minute: None,
            queue_quota: None,
            context_budget_tokens: None,
        }
    }

    pub fn with_agent_id(mut self, agent_id: u64) -> Self {
        self.agent_id = Some(agent_id);
        self
    }

    pub fn with_principal_id(mut self, principal_id: impl Into<String>) -> Self {
        self.principal_id = Some(principal_id.into());
        self
    }

    pub fn with_request_quota_per_minute(mut self, quota: u64) -> Self {
        self.request_quota_per_minute = Some(quota);
        self
    }

    pub fn with_body_quota_bytes_per_minute(mut self, quota: u64) -> Self {
        self.body_quota_bytes_per_minute = Some(quota);
        self
    }

    pub fn with_queue_quota(mut self, quota: u64) -> Self {
        self.queue_quota = Some(quota);
        self
    }

    pub fn with_context_budget_tokens(mut self, budget: u32) -> Self {
        self.context_budget_tokens = Some(budget);
        self
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AuthRouteContext {
    pub agent_id: Option<u64>,
    pub context_budget_tokens: Option<u32>,
    pub principal_id: Option<String>,
    pub role: Option<AuthRole>,
}

impl AuthRouteContext {
    pub fn for_agent(agent_id: Option<u64>) -> Self {
        Self {
            agent_id,
            context_budget_tokens: None,
            principal_id: None,
            role: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct AuthDecision {
    pub agent_id: Option<u64>,
    pub role: Option<AuthRole>,
    pub principal_id: Option<String>,
    pub tenants: Option<std::collections::BTreeSet<String>>,
    pub quota_key: Option<String>,
    pub request_quota_per_minute: Option<u64>,
    pub body_quota_bytes_per_minute: Option<u64>,
    pub queue_quota: Option<u64>,
    pub context_budget_tokens: Option<u32>,
}

impl AuthDecision {
    pub(crate) fn route_context(&self) -> AuthRouteContext {
        AuthRouteContext {
            agent_id: self.agent_id,
            context_budget_tokens: self.context_budget_tokens,
            principal_id: self.principal_id.clone(),
            role: self.role,
        }
    }
}
