use std::path::Path;

use serde::Serialize;

use crate::auth::AuthTokenPolicy;
use crate::auth_capability::EffectiveAuthPolicy;
use crate::responses::RouterError;
use crate::router::query_param_decoded;
use crate::ServerOptions;

use super::io::{encode_store_json, policy_path, read_store};
use super::mutations::{disable_principal, list_policies, rollback_policy, upsert_principal};
use super::types::{AuthPolicyAdminResponse, AuthPolicyMutationRequest};
use super::validation::{parse_capabilities, parse_role, parse_tenants, validate_store};

pub(crate) fn load_token_policies_from_store(
    path: &Path,
) -> Result<Vec<EffectiveAuthPolicy>, String> {
    let store = read_store(path)?;
    let mut policies = Vec::new();
    for principal in validate_store(store)?.principals {
        if principal.disabled {
            continue;
        }
        let role = parse_role(&principal.role)?;
        let mut policy = EffectiveAuthPolicy::from_token_policy(
            AuthTokenPolicy::new(principal.token, role).with_principal_id(principal.principal_id),
        );
        if let Some(agent_id) = principal.agent_id {
            policy = policy.with_agent_id(agent_id);
        }
        if let Some(quota) = principal.request_quota_per_minute {
            policy = policy.with_request_quota_per_minute(quota);
        }
        if let Some(quota) = principal.body_quota_bytes_per_minute {
            policy = policy.with_body_quota_bytes_per_minute(quota);
        }
        if let Some(quota) = principal.queue_quota {
            policy = policy.with_queue_quota(quota);
        }
        if let Some(budget) = principal.context_budget_tokens {
            policy = policy.with_context_budget_tokens(budget);
        }
        if let Some(capabilities) = principal.capabilities {
            policy = policy.with_capabilities(parse_capabilities(&capabilities)?);
        }
        if let Some(tenants) = principal.tenants {
            policy = policy.with_tenants(parse_tenants(&tenants)?);
        }
        policies.push(policy);
    }
    Ok(policies)
}

pub(crate) fn handle_admin_request(
    options: &ServerOptions,
    method: &str,
    path: &str,
    query: &str,
    body: &[u8],
) -> Result<Option<AuthPolicyAdminResponse>, RouterError> {
    match (method, path) {
        ("POST", "/v1/admin/auth/principal") => {
            let request =
                serde_json::from_slice::<AuthPolicyMutationRequest>(body).map_err(|error| {
                    RouterError::BadRequest(format!("invalid auth policy JSON: {error}"))
                })?;
            let path = policy_path(options)?;
            Ok(Some(admin_response(
                path,
                &upsert_principal(path, request)?,
                true,
            )?))
        }
        ("DELETE", "/v1/admin/auth/principal") => {
            let principal_id =
                query_param_decoded(query, "principal_id").map_err(RouterError::BadRequest)?;
            let path = policy_path(options)?;
            Ok(Some(admin_response(
                path,
                &disable_principal(path, &principal_id)?,
                true,
            )?))
        }
        ("POST", "/v1/admin/auth/policy/rollback") => {
            let path = policy_path(options)?;
            Ok(Some(admin_response(path, &rollback_policy(path)?, true)?))
        }
        ("GET", "/v1/admin/auth/policies") => {
            let path = policy_path(options)?;
            Ok(Some(admin_response(path, &list_policies(path)?, false)?))
        }
        _ => Ok(None),
    }
}

fn admin_response<T: Serialize>(
    path: &Path,
    value: &T,
    sync_policy_cells: bool,
) -> Result<AuthPolicyAdminResponse, RouterError> {
    let body = serde_json::to_string(value)?;
    let store = read_store(path).map_err(RouterError::BadRequest)?;
    let policy_store_json = encode_store_json(&store).map_err(RouterError::Internal)?;
    Ok(AuthPolicyAdminResponse {
        body,
        policy_store_json,
        sync_policy_cells,
    })
}
