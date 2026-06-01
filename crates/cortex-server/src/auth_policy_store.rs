use crate::auth::{AuthRole, AuthTokenPolicy};
use crate::auth_policy_io::{atomic_write_text, rollback_path};
use crate::responses::RouterError;
use crate::router::query_param_decoded;
use crate::ServerOptions;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

const SCHEMA_VERSION: &str = "cortexdb.auth_policy.v1";
const LEGACY_SCHEMA_VERSION_V0: &str = "cortexdb.auth_policy.v0";

#[derive(Clone, Debug, Deserialize, Serialize)]
struct AuthPolicyStoreFile {
    schema_version: String,
    principals: Vec<AuthPolicyPrincipal>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct AuthPolicyPrincipal {
    principal_id: String,
    token: String,
    role: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    agent_id: Option<u64>,
    #[serde(default, skip_serializing_if = "is_false")]
    disabled: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    request_quota_per_minute: Option<u64>,
}

#[derive(Clone, Debug, Deserialize)]
struct AuthPolicyStoreFileV0 {
    schema_version: String,
    tokens: Vec<AuthPolicyPrincipalV0>,
}

#[derive(Clone, Debug, Deserialize)]
struct AuthPolicyPrincipalV0 {
    principal_id: String,
    token: String,
    role: String,
    #[serde(default)]
    agent_id: Option<u64>,
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

pub(crate) fn load_token_policies_from_store(path: &Path) -> Result<Vec<AuthTokenPolicy>, String> {
    let store = read_store(path)?;
    let mut policies = Vec::new();
    for principal in validate_store(store)?.principals {
        if principal.disabled {
            continue;
        }
        let role = parse_role(&principal.role)?;
        let mut policy =
            AuthTokenPolicy::new(principal.token, role).with_principal_id(principal.principal_id);
        if let Some(agent_id) = principal.agent_id {
            policy = policy.with_agent_id(agent_id);
        }
        if let Some(quota) = principal.request_quota_per_minute {
            policy = policy.with_request_quota_per_minute(quota);
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
) -> Result<Option<String>, RouterError> {
    let response = match (method, path) {
        ("POST", "/v1/admin/auth/principal") => {
            let request =
                serde_json::from_slice::<AuthPolicyMutationRequest>(body).map_err(|error| {
                    RouterError::BadRequest(format!("invalid auth policy JSON: {error}"))
                })?;
            Some(upsert_principal(policy_path(options)?, request)?)
        }
        ("DELETE", "/v1/admin/auth/principal") => {
            let principal_id =
                query_param_decoded(query, "principal_id").map_err(RouterError::BadRequest)?;
            Some(disable_principal(policy_path(options)?, &principal_id)?)
        }
        ("POST", "/v1/admin/auth/policy/rollback") => Some(rollback_policy(policy_path(options)?)?),
        _ => None,
    };
    response
        .map(|value| serde_json::to_string(&value).map_err(Into::into))
        .transpose()
}

fn upsert_principal(
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

fn disable_principal(
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

fn rollback_policy(path: &Path) -> Result<AuthPolicyMutationResponse, RouterError> {
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

fn policy_path(options: &ServerOptions) -> Result<&Path, RouterError> {
    options.auth_policy_store_file.as_deref().ok_or_else(|| {
        RouterError::BadRequest("auth policy store file is not configured".to_owned())
    })
}

fn read_store_or_empty(path: &Path) -> Result<AuthPolicyStoreFile, RouterError> {
    if path.is_file() {
        read_store(path).map_err(RouterError::BadRequest)
    } else {
        Ok(empty_store())
    }
}

fn read_store(path: &Path) -> Result<AuthPolicyStoreFile, String> {
    let raw = fs::read_to_string(path)
        .map_err(|error| format!("auth policy store could not be read: {error}"))?;
    let value = serde_json::from_str::<serde_json::Value>(&raw)
        .map_err(|error| format!("auth policy store is invalid JSON: {error}"))?;
    decode_store_value(value)
}

fn decode_store_value(value: serde_json::Value) -> Result<AuthPolicyStoreFile, String> {
    let schema_version = value
        .get("schema_version")
        .and_then(|value| value.as_str())
        .ok_or_else(|| "auth policy store schema_version is required".to_owned())?;
    match schema_version {
        SCHEMA_VERSION => {
            let store = serde_json::from_value::<AuthPolicyStoreFile>(value)
                .map_err(|error| format!("auth policy store v1 is invalid: {error}"))?;
            validate_store(store)
        }
        LEGACY_SCHEMA_VERSION_V0 => {
            let legacy = serde_json::from_value::<AuthPolicyStoreFileV0>(value)
                .map_err(|error| format!("auth policy store v0 is invalid: {error}"))?;
            validate_store(migrate_v0_store(legacy)?)
        }
        _ => Err(
            "auth policy store schema_version must be cortexdb.auth_policy.v1 or cortexdb.auth_policy.v0"
                .to_owned(),
        ),
    }
}

fn migrate_v0_store(legacy: AuthPolicyStoreFileV0) -> Result<AuthPolicyStoreFile, String> {
    if legacy.schema_version != LEGACY_SCHEMA_VERSION_V0 {
        return Err("auth policy store v0 schema_version is invalid".to_owned());
    }
    Ok(AuthPolicyStoreFile {
        schema_version: SCHEMA_VERSION.to_owned(),
        principals: legacy
            .tokens
            .into_iter()
            .map(|token| AuthPolicyPrincipal {
                principal_id: token.principal_id,
                token: token.token,
                role: token.role,
                agent_id: token.agent_id,
                disabled: false,
                request_quota_per_minute: None,
            })
            .collect(),
    })
}

fn validate_store(store: AuthPolicyStoreFile) -> Result<AuthPolicyStoreFile, String> {
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

fn validate_principal(principal: &AuthPolicyPrincipal) -> Result<(), String> {
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
    Ok(())
}

fn parse_role(raw: &str) -> Result<AuthRole, String> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "admin" => Ok(AuthRole::Admin),
        "data" => Ok(AuthRole::Data),
        _ => Err("auth token role must be admin or data".to_owned()),
    }
}

fn empty_store() -> AuthPolicyStoreFile {
    AuthPolicyStoreFile {
        schema_version: SCHEMA_VERSION.to_owned(),
        principals: Vec::new(),
    }
}

fn atomic_write_json(path: &Path, store: &AuthPolicyStoreFile) -> Result<(), String> {
    let text = serde_json::to_string_pretty(store)
        .map_err(|error| format!("failed to encode auth policy store: {error}"))?;
    atomic_write_text(path, &(text + "\n"))
}

fn is_false(value: &bool) -> bool {
    !*value
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_v0_policy_store_migrates_to_v1_principals() {
        let value = serde_json::json!({
            "schema_version": "cortexdb.auth_policy.v0",
            "tokens": [
                {"principal_id": "data-a", "token": "secret", "role": "data", "agent_id": 7}
            ]
        });

        let store = decode_store_value(value).expect("legacy store should migrate");

        assert_eq!(store.schema_version, SCHEMA_VERSION);
        assert_eq!(store.principals.len(), 1);
        assert_eq!(store.principals[0].principal_id, "data-a");
        assert_eq!(store.principals[0].agent_id, Some(7));
        assert!(!store.principals[0].disabled);
    }

    #[test]
    fn unsupported_policy_store_schema_fails_closed() {
        let value = serde_json::json!({
            "schema_version": "cortexdb.auth_policy.v9",
            "principals": []
        });

        let error = decode_store_value(value).unwrap_err();

        assert!(error
            .contains("schema_version must be cortexdb.auth_policy.v1 or cortexdb.auth_policy.v0"));
    }
}
