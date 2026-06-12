use std::fs;
use std::path::Path;

use crate::auth_policy_io::atomic_write_text;
use crate::responses::RouterError;
use crate::ServerOptions;

use super::types::{AuthPolicyPrincipal, AuthPolicyStoreFile, AuthPolicyStoreFileV0};
use super::validation::validate_store;
use super::{LEGACY_SCHEMA_VERSION_V0, SCHEMA_VERSION};

pub(super) fn policy_path(options: &ServerOptions) -> Result<&Path, RouterError> {
    options.auth_policy_store_file.as_deref().ok_or_else(|| {
        RouterError::BadRequest("auth policy store file is not configured".to_owned())
    })
}

pub(super) fn read_store_or_empty(path: &Path) -> Result<AuthPolicyStoreFile, RouterError> {
    if path.is_file() {
        read_store(path).map_err(RouterError::BadRequest)
    } else {
        Ok(empty_store())
    }
}

pub(super) fn read_store(path: &Path) -> Result<AuthPolicyStoreFile, String> {
    let raw = fs::read_to_string(path)
        .map_err(|error| format!("auth policy store could not be read: {error}"))?;
    decode_store_str(&raw)
}

pub(crate) fn decode_store_str(raw: &str) -> Result<AuthPolicyStoreFile, String> {
    let value = serde_json::from_str::<serde_json::Value>(raw)
        .map_err(|error| format!("auth policy store is invalid JSON: {error}"))?;
    decode_store_value(value)
}

pub(super) fn decode_store_value(value: serde_json::Value) -> Result<AuthPolicyStoreFile, String> {
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
                body_quota_bytes_per_minute: None,
                queue_quota: None,
                context_budget_tokens: None,
                capabilities: None,
                tenants: None,
            })
            .collect(),
    })
}

pub(super) fn empty_store() -> AuthPolicyStoreFile {
    AuthPolicyStoreFile {
        schema_version: SCHEMA_VERSION.to_owned(),
        principals: Vec::new(),
    }
}

pub(super) fn atomic_write_json(path: &Path, store: &AuthPolicyStoreFile) -> Result<(), String> {
    let text = encode_store_json(store)?;
    atomic_write_text(path, &(text + "\n"))
}

pub(super) fn encode_store_json(store: &AuthPolicyStoreFile) -> Result<String, String> {
    serde_json::to_string_pretty(store)
        .map_err(|error| format!("failed to encode auth policy store: {error}"))
}
