use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct AuthPolicyStoreFile {
    pub(crate) schema_version: String,
    pub(crate) principals: Vec<AuthPolicyPrincipal>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct AuthPolicyPrincipal {
    pub(crate) principal_id: String,
    pub(crate) token: String,
    pub(crate) role: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) agent_id: Option<u64>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub(crate) disabled: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) request_quota_per_minute: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) body_quota_bytes_per_minute: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) queue_quota: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) context_budget_tokens: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) capabilities: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) tenants: Option<Vec<String>>,
}

#[derive(Clone, Debug, Deserialize)]
pub(super) struct AuthPolicyStoreFileV0 {
    pub(super) schema_version: String,
    pub(super) tokens: Vec<AuthPolicyPrincipalV0>,
}

#[derive(Clone, Debug, Deserialize)]
pub(super) struct AuthPolicyPrincipalV0 {
    pub(super) principal_id: String,
    pub(super) token: String,
    pub(super) role: String,
    #[serde(default)]
    pub(super) agent_id: Option<u64>,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct AuthPolicyMutationRequest {
    pub principal_id: String,
    pub token: String,
    pub role: String,
    #[serde(default)]
    pub agent_id: Option<u64>,
    #[serde(default)]
    pub disabled: bool,
    #[serde(default)]
    pub request_quota_per_minute: Option<u64>,
    #[serde(default)]
    pub body_quota_bytes_per_minute: Option<u64>,
    #[serde(default)]
    pub queue_quota: Option<u64>,
    #[serde(default)]
    pub context_budget_tokens: Option<u32>,
    #[serde(default)]
    pub capabilities: Option<Vec<String>>,
    #[serde(default)]
    pub tenants: Option<Vec<String>>,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct AuthPolicyMutationResponse {
    pub schema_version: &'static str,
    pub action: &'static str,
    pub principal_id: Option<String>,
    pub active_principals: usize,
    pub disabled_principals: usize,
    pub rollback_available: bool,
}

#[derive(Clone, Debug, Serialize)]
pub(super) struct AuthPolicyListResponse {
    pub(super) schema_version: &'static str,
    pub(super) supported_roles: [&'static str; 2],
    pub(super) principal_count: usize,
    pub(super) active_principals: usize,
    pub(super) disabled_principals: usize,
    pub(super) principals: Vec<AuthPolicyListPrincipal>,
    pub(super) token_redaction: &'static str,
}

#[derive(Clone, Debug, Serialize)]
pub(super) struct AuthPolicyListPrincipal {
    pub(super) principal_id: String,
    pub(super) role: String,
    pub(super) agent_id: Option<u64>,
    pub(super) disabled: bool,
    pub(super) request_quota_per_minute: Option<u64>,
    pub(super) body_quota_bytes_per_minute: Option<u64>,
    pub(super) queue_quota: Option<u64>,
    pub(super) context_budget_tokens: Option<u32>,
    pub(super) capabilities: Vec<String>,
    pub(super) tenants: Vec<String>,
    pub(super) token_present: bool,
    pub(super) token_fingerprint: String,
}

pub(crate) struct AuthPolicyAdminResponse {
    pub body: String,
    pub policy_store_json: String,
    pub sync_policy_cells: bool,
}

fn is_false(value: &bool) -> bool {
    !*value
}
