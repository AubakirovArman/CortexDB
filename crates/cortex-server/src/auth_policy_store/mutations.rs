use std::fs;
use std::path::Path;

use crate::auth_policy_io::{atomic_write_text, rollback_path};
use crate::responses::RouterError;

use super::io::{atomic_write_json, empty_store, read_store, read_store_or_empty};
use super::types::{
    AuthPolicyListPrincipal, AuthPolicyListResponse, AuthPolicyMutationRequest,
    AuthPolicyMutationResponse, AuthPolicyPrincipal, AuthPolicyStoreFile,
};
use super::validation::{
    canonical_capabilities, canonical_tenants, validate_principal, validate_store,
};

pub(super) fn upsert_principal(
    path: &Path,
    request: AuthPolicyMutationRequest,
) -> Result<AuthPolicyMutationResponse, RouterError> {
    let mut store = read_store_or_empty(path)?;
    let principal = AuthPolicyPrincipal {
        principal_id: request.principal_id.trim().to_owned(),
        token: request.token.trim().to_owned(),
        role: request.role.trim().to_ascii_lowercase(),
        agent_id: request.agent_id,
        disabled: request.disabled,
        request_quota_per_minute: request.request_quota_per_minute,
        body_quota_bytes_per_minute: request.body_quota_bytes_per_minute,
        queue_quota: request.queue_quota,
        context_budget_tokens: request.context_budget_tokens,
        capabilities: request.capabilities,
        tenants: request.tenants,
    };
    validate_principal(&principal).map_err(RouterError::BadRequest)?;
    if let Some(existing) = store
        .principals
        .iter_mut()
        .find(|existing| existing.principal_id == principal.principal_id)
    {
        *existing = principal.clone();
    } else {
        store.principals.push(principal.clone());
    }
    persist_mutated_store(path, store)?;
    Ok(response(
        "upsert_principal",
        Some(principal.principal_id),
        path,
    ))
}

pub(super) fn disable_principal(
    path: &Path,
    principal_id: &str,
) -> Result<AuthPolicyMutationResponse, RouterError> {
    let mut store = read_store(path).map_err(RouterError::BadRequest)?;
    let target = principal_id.trim();
    if target.is_empty() {
        return Err(RouterError::BadRequest(
            "principal_id must not be empty".to_owned(),
        ));
    }
    let Some(principal) = store
        .principals
        .iter_mut()
        .find(|principal| principal.principal_id == target)
    else {
        return Err(RouterError::NotFound("principal not found".to_owned()));
    };
    principal.disabled = true;
    persist_mutated_store(path, store)?;
    Ok(response("disable_principal", Some(target.to_owned()), path))
}

pub(super) fn rollback_policy(path: &Path) -> Result<AuthPolicyMutationResponse, RouterError> {
    let rollback_path = rollback_path(path);
    if !rollback_path.is_file() {
        return Err(RouterError::NotFound(
            "auth policy rollback snapshot not found".to_owned(),
        ));
    }
    let store = read_store(&rollback_path).map_err(RouterError::BadRequest)?;
    atomic_write_json(path, &validate_store(store)?).map_err(RouterError::Internal)?;
    Ok(response("rollback_policy", None, path))
}

pub(super) fn list_policies(path: &Path) -> Result<AuthPolicyListResponse, RouterError> {
    let store = read_store(path).map_err(RouterError::BadRequest)?;
    let principals = store
        .principals
        .iter()
        .map(redacted_principal)
        .collect::<Result<Vec<_>, _>>()?;
    let active_principals = store
        .principals
        .iter()
        .filter(|principal| !principal.disabled)
        .count();
    let disabled_principals = store.principals.len().saturating_sub(active_principals);
    Ok(AuthPolicyListResponse {
        schema_version: "cortexdb.auth_policy_list.v1",
        supported_roles: ["admin", "data"],
        principal_count: store.principals.len(),
        active_principals,
        disabled_principals,
        principals,
        token_redaction: "token omitted; token_fingerprint uses stable fnv64",
    })
}

fn redacted_principal(
    principal: &AuthPolicyPrincipal,
) -> Result<AuthPolicyListPrincipal, RouterError> {
    let capabilities = principal
        .capabilities
        .as_deref()
        .map(canonical_capabilities)
        .transpose()?
        .unwrap_or_default();
    let tenants = principal
        .tenants
        .as_deref()
        .map(canonical_tenants)
        .transpose()?
        .unwrap_or_default();
    Ok(AuthPolicyListPrincipal {
        principal_id: principal.principal_id.clone(),
        role: principal.role.trim().to_ascii_lowercase(),
        agent_id: principal.agent_id,
        disabled: principal.disabled,
        request_quota_per_minute: principal.request_quota_per_minute,
        body_quota_bytes_per_minute: principal.body_quota_bytes_per_minute,
        queue_quota: principal.queue_quota,
        context_budget_tokens: principal.context_budget_tokens,
        capabilities,
        tenants,
        token_present: !principal.token.trim().is_empty(),
        token_fingerprint: token_fingerprint(&principal.token),
    })
}

fn persist_mutated_store(path: &Path, store: AuthPolicyStoreFile) -> Result<(), RouterError> {
    let store = validate_store(store).map_err(RouterError::BadRequest)?;
    if path.is_file() {
        let current = fs::read_to_string(path).map_err(|error| {
            RouterError::Internal(format!("failed to read policy store: {error}"))
        })?;
        atomic_write_text(&rollback_path(path), &current).map_err(RouterError::Internal)?;
    }
    atomic_write_json(path, &store).map_err(RouterError::Internal)
}

fn response(
    action: &'static str,
    principal_id: Option<String>,
    path: &Path,
) -> AuthPolicyMutationResponse {
    let store = read_store(path).unwrap_or_else(|_| empty_store());
    let active_principals = store
        .principals
        .iter()
        .filter(|principal| !principal.disabled)
        .count();
    let disabled_principals = store.principals.len().saturating_sub(active_principals);
    AuthPolicyMutationResponse {
        schema_version: "cortexdb.auth_policy_mutation.v1",
        action,
        principal_id,
        active_principals,
        disabled_principals,
        rollback_available: rollback_path(path).is_file(),
    }
}

fn token_fingerprint(token: &str) -> String {
    format!("fnv64:{:016x}", stable_hash(token))
}

fn stable_hash(value: &str) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for byte in value.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}
