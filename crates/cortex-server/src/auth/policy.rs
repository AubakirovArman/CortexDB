use super::parser::effective_auth_policies;
use super::routes::{capabilities_can_access, role_can_access};
use super::AuthDecision;
use crate::responses::RouterError;
use crate::ServerOptions;

pub(crate) fn authorize_request(
    options: &ServerOptions,
    auth_header: Option<&str>,
    method: &str,
    path: &str,
) -> Result<AuthDecision, RouterError> {
    let Some(policy) = matching_policy(options, auth_header)? else {
        if has_auth(options) {
            return Err(RouterError::Unauthorized);
        }
        return Ok(AuthDecision {
            agent_id: None,
            role: None,
            principal_id: None,
            tenants: None,
            quota_key: None,
            request_quota_per_minute: None,
            body_quota_bytes_per_minute: None,
            queue_quota: None,
            context_budget_tokens: None,
        });
    };

    if !role_can_access(policy.role, method, path) {
        return Err(RouterError::Forbidden(
            "token role is not allowed to access this route".to_owned(),
        ));
    }
    if !capabilities_can_access(&policy, method, path) {
        return Err(RouterError::Forbidden(
            "token capability is not allowed to access this route".to_owned(),
        ));
    }
    Ok(AuthDecision {
        agent_id: policy.agent_id,
        role: Some(policy.role),
        principal_id: policy.principal_id.clone(),
        tenants: policy.tenants.clone(),
        quota_key: Some(policy.quota_key()),
        request_quota_per_minute: policy.request_quota_per_minute,
        body_quota_bytes_per_minute: policy.body_quota_bytes_per_minute,
        queue_quota: policy.queue_quota,
        context_budget_tokens: policy.context_budget_tokens,
    })
}

pub(crate) fn tenant_can_access(decision: &AuthDecision, tenant: &str) -> bool {
    match &decision.tenants {
        Some(tenants) => tenants.contains(tenant),
        None => true,
    }
}

pub(crate) fn validate_token_policies(options: &ServerOptions) -> Result<(), String> {
    for policy in effective_auth_policies(options)? {
        if policy.token.trim().is_empty() {
            return Err("auth token policy contains an empty token".to_owned());
        }
        if matches!(policy.agent_id, Some(0)) {
            return Err("auth token policy agent_id must be greater than zero".to_owned());
        }
        if matches!(policy.principal_id.as_deref(), Some("")) {
            return Err("auth token policy principal_id must not be empty".to_owned());
        }
        if matches!(policy.request_quota_per_minute, Some(0)) {
            return Err(
                "auth token policy request_quota_per_minute must be greater than zero".to_owned(),
            );
        }
        if matches!(policy.body_quota_bytes_per_minute, Some(0)) {
            return Err(
                "auth token policy body_quota_bytes_per_minute must be greater than zero"
                    .to_owned(),
            );
        }
        if matches!(policy.queue_quota, Some(0)) {
            return Err("auth token policy queue_quota must be greater than zero".to_owned());
        }
        if matches!(policy.context_budget_tokens, Some(0)) {
            return Err(
                "auth token policy context_budget_tokens must be greater than zero".to_owned(),
            );
        }
    }
    Ok(())
}

fn matching_policy(
    options: &ServerOptions,
    auth_header: Option<&str>,
) -> Result<Option<crate::auth_capability::EffectiveAuthPolicy>, RouterError> {
    let Some(bearer) = auth_header.and_then(|value| value.strip_prefix("Bearer ")) else {
        return Ok(None);
    };
    let tokens = effective_auth_policies(options).map_err(|_| {
        RouterError::Internal("auth token policy file could not be read".to_owned())
    })?;
    Ok(tokens.into_iter().find(|policy| policy.token == bearer))
}

fn has_auth(options: &ServerOptions) -> bool {
    options.auth_token.is_some()
        || !options.auth_tokens.is_empty()
        || options.auth_tokens_file.is_some()
        || options.auth_policy_store_file.is_some()
}
