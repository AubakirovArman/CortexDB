use std::collections::BTreeSet;

use crate::auth::AuthRole;
use crate::auth_capability::AuthCapability;
use crate::responses::RouterError;
use crate::validate_tenant_id;

use super::types::{AuthPolicyPrincipal, AuthPolicyStoreFile};
use super::SCHEMA_VERSION;

pub(super) fn validate_store(store: AuthPolicyStoreFile) -> Result<AuthPolicyStoreFile, String> {
    if store.schema_version != SCHEMA_VERSION {
        return Err("auth policy store schema_version must be cortexdb.auth_policy.v1".to_owned());
    }
    let mut seen_principals = BTreeSet::new();
    let mut seen_tokens = BTreeSet::new();
    for (index, principal) in store.principals.iter().enumerate() {
        validate_principal(principal)
            .map_err(|error| format!("auth policy store principal {} {error}", index + 1))?;
        if !seen_principals.insert(principal.principal_id.clone()) {
            return Err(format!(
                "auth policy store principal_id {:?} is duplicated",
                principal.principal_id
            ));
        }
        if !seen_tokens.insert(principal.token.clone()) {
            return Err(format!(
                "auth policy store token for principal {:?} is duplicated",
                principal.principal_id
            ));
        }
    }
    Ok(store)
}

pub(super) fn validate_principal(principal: &AuthPolicyPrincipal) -> Result<(), String> {
    if principal.principal_id.trim().is_empty() {
        return Err("has empty principal_id".to_owned());
    }
    if principal.token.trim().is_empty() {
        return Err("has empty token".to_owned());
    }
    parse_role(&principal.role)?;
    if matches!(principal.agent_id, Some(0)) {
        return Err("has invalid agent_id".to_owned());
    }
    if matches!(principal.request_quota_per_minute, Some(0)) {
        return Err("has invalid request_quota_per_minute".to_owned());
    }
    if matches!(principal.body_quota_bytes_per_minute, Some(0)) {
        return Err("has invalid body_quota_bytes_per_minute".to_owned());
    }
    if matches!(principal.queue_quota, Some(0)) {
        return Err("has invalid queue_quota".to_owned());
    }
    if matches!(principal.context_budget_tokens, Some(0)) {
        return Err("has invalid context_budget_tokens".to_owned());
    }
    if let Some(capabilities) = &principal.capabilities {
        parse_capabilities(capabilities)?;
    }
    if let Some(tenants) = &principal.tenants {
        parse_tenants(tenants)?;
    }
    Ok(())
}

pub(crate) fn parse_capabilities(raw: &[String]) -> Result<BTreeSet<AuthCapability>, String> {
    if raw.is_empty() {
        return Err("has empty capabilities".to_owned());
    }
    let mut capabilities = BTreeSet::new();
    for value in raw {
        let capability = AuthCapability::parse(value)?;
        if !capabilities.insert(capability) {
            return Err("has duplicate capability".to_owned());
        }
    }
    Ok(capabilities)
}

pub(crate) fn parse_tenants(raw: &[String]) -> Result<BTreeSet<String>, String> {
    if raw.is_empty() {
        return Err("has empty tenants".to_owned());
    }
    let mut tenants = BTreeSet::new();
    for value in raw {
        let tenant = value.trim();
        if !validate_tenant_id(tenant) {
            return Err("has invalid tenant".to_owned());
        }
        if !tenants.insert(tenant.to_owned()) {
            return Err("has duplicate tenant".to_owned());
        }
    }
    Ok(tenants)
}

pub(super) fn canonical_capabilities(raw: &[String]) -> Result<Vec<String>, RouterError> {
    Ok(parse_capabilities(raw)
        .map_err(RouterError::BadRequest)?
        .into_iter()
        .map(|capability| capability.as_str().to_owned())
        .collect())
}

pub(super) fn canonical_tenants(raw: &[String]) -> Result<Vec<String>, RouterError> {
    Ok(parse_tenants(raw)
        .map_err(RouterError::BadRequest)?
        .into_iter()
        .collect())
}

pub(super) fn parse_role(raw: &str) -> Result<AuthRole, String> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "admin" => Ok(AuthRole::Admin),
        "data" => Ok(AuthRole::Data),
        _ => Err("auth token role must be admin or data".to_owned()),
    }
}
