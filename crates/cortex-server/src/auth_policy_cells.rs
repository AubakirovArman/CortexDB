use cortex_core::CellId;
use cortex_engine::Database;
use serde::{Deserialize, Serialize};

#[cfg(test)]
use crate::auth::AuthRole;
#[cfg(test)]
use crate::auth_capability::EffectiveAuthPolicy;
use crate::auth_policy_store::{self, AuthPolicyPrincipal, AuthPolicyStoreFile};
use crate::responses::RouterError;

const AUTH_POLICY_SCOPE: &str = "_system:auth_policy";
const AUTH_POLICY_NAMESPACE: u64 = 0xA710_0000_0000_0000;
const MANIFEST_KEY: &str = "__auth_policy_manifest__";

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct AuthPolicyCellSyncReport {
    pub principals_synced: usize,
    pub manifest_cell_id: CellId,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub(crate) struct AuthPolicyCellRecord {
    pub principal_id: String,
    pub role: String,
    pub agent_id: Option<u64>,
    pub disabled: bool,
    pub request_quota_per_minute: Option<u64>,
    pub body_quota_bytes_per_minute: Option<u64>,
    pub queue_quota: Option<u64>,
    pub capabilities: Vec<String>,
    pub token_fingerprint: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
struct AuthPolicyCellManifest {
    schema_version: String,
    principal_ids: Vec<String>,
}

pub(crate) fn sync_store_json_to_database(
    db: &mut Database,
    raw: &str,
) -> Result<AuthPolicyCellSyncReport, RouterError> {
    let store = auth_policy_store::decode_store_str(raw).map_err(RouterError::BadRequest)?;
    sync_store_to_database(db, &store)
}

pub(crate) fn sync_store_to_database(
    db: &mut Database,
    store: &AuthPolicyStoreFile,
) -> Result<AuthPolicyCellSyncReport, RouterError> {
    let mut principal_ids = Vec::with_capacity(store.principals.len());
    for principal in &store.principals {
        let record = record_from_principal(principal)?;
        db.put_cell(
            policy_cell_id(&principal.principal_id),
            encode_record(&record)?,
        )
        .map_err(|error| RouterError::Internal(error.to_string()))?;
        principal_ids.push(principal.principal_id.clone());
    }
    principal_ids.sort();

    let manifest = AuthPolicyCellManifest {
        schema_version: store.schema_version.clone(),
        principal_ids,
    };
    let manifest_cell_id = policy_cell_id(MANIFEST_KEY);
    db.put_cell(manifest_cell_id, encode_manifest(&manifest)?)
        .map_err(|error| RouterError::Internal(error.to_string()))?;

    Ok(AuthPolicyCellSyncReport {
        principals_synced: store.principals.len(),
        manifest_cell_id,
    })
}

#[cfg(test)]
pub(crate) fn load_policy_cell_records(
    db: &Database,
) -> Result<Vec<AuthPolicyCellRecord>, RouterError> {
    let Some(manifest_payload) = db.get_latest_cell(policy_cell_id(MANIFEST_KEY)) else {
        return Ok(Vec::new());
    };
    let manifest = decode_body_json::<AuthPolicyCellManifest>(&manifest_payload)?;
    let mut records = Vec::with_capacity(manifest.principal_ids.len());
    for principal_id in manifest.principal_ids {
        let cell_id = policy_cell_id(&principal_id);
        let Some(payload) = db.get_latest_cell(cell_id) else {
            return Err(RouterError::Internal(format!(
                "auth policy cell missing for principal {principal_id:?}"
            )));
        };
        records.push(decode_body_json::<AuthPolicyCellRecord>(&payload)?);
    }
    records.sort_by(|left, right| left.principal_id.cmp(&right.principal_id));
    Ok(records)
}

#[cfg(test)]
pub(crate) fn effective_policy_mapping_from_cells(
    db: &Database,
) -> Result<Vec<EffectiveAuthPolicy>, RouterError> {
    let mut policies = Vec::new();
    for record in load_policy_cell_records(db)? {
        if record.disabled {
            continue;
        }
        let role = parse_role(&record.role)?;
        let mut policy = EffectiveAuthPolicy {
            token: record.token_fingerprint,
            role,
            agent_id: record.agent_id,
            principal_id: Some(record.principal_id),
            request_quota_per_minute: record.request_quota_per_minute,
            body_quota_bytes_per_minute: record.body_quota_bytes_per_minute,
            queue_quota: record.queue_quota,
            capabilities: None,
        };
        if !record.capabilities.is_empty() {
            policy = policy.with_capabilities(
                auth_policy_store::parse_capabilities(&record.capabilities)
                    .map_err(RouterError::BadRequest)?,
            );
        }
        policies.push(policy);
    }
    Ok(policies)
}

fn record_from_principal(
    principal: &AuthPolicyPrincipal,
) -> Result<AuthPolicyCellRecord, RouterError> {
    let capabilities = principal
        .capabilities
        .as_deref()
        .map(canonical_capabilities)
        .transpose()?
        .unwrap_or_default();
    Ok(AuthPolicyCellRecord {
        principal_id: principal.principal_id.clone(),
        role: principal.role.trim().to_ascii_lowercase(),
        agent_id: principal.agent_id,
        disabled: principal.disabled,
        request_quota_per_minute: principal.request_quota_per_minute,
        body_quota_bytes_per_minute: principal.body_quota_bytes_per_minute,
        queue_quota: principal.queue_quota,
        capabilities,
        token_fingerprint: token_fingerprint(&principal.token),
    })
}

fn canonical_capabilities(raw: &[String]) -> Result<Vec<String>, RouterError> {
    let parsed = auth_policy_store::parse_capabilities(raw).map_err(RouterError::BadRequest)?;
    Ok(parsed
        .into_iter()
        .map(|capability| capability.as_str().to_owned())
        .collect())
}

#[cfg(test)]
fn parse_role(raw: &str) -> Result<AuthRole, RouterError> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "admin" => Ok(AuthRole::Admin),
        "data" => Ok(AuthRole::Data),
        _ => Err(RouterError::BadRequest(
            "auth policy cell has invalid role".to_owned(),
        )),
    }
}

fn encode_record(record: &AuthPolicyCellRecord) -> Result<Vec<u8>, RouterError> {
    encode_payload(
        "auth_policy_principal",
        &format!("Auth policy principal {}", record.principal_id),
        record,
    )
}

fn encode_manifest(manifest: &AuthPolicyCellManifest) -> Result<Vec<u8>, RouterError> {
    encode_payload("auth_policy_manifest", "Auth policy manifest", manifest)
}

fn encode_payload<T: Serialize>(
    cell_type: &str,
    title: &str,
    body: &T,
) -> Result<Vec<u8>, RouterError> {
    let body = serde_json::to_string_pretty(body).map_err(|error| {
        RouterError::Internal(format!("auth policy cell serialization failed: {error}"))
    })?;
    Ok(format!(
        "scope={AUTH_POLICY_SCOPE}\nstatus=ready\ntype={cell_type}\ntitle={title}\n\n{body}\n"
    )
    .into_bytes())
}

#[cfg(test)]
fn decode_body_json<T: for<'de> Deserialize<'de>>(payload: &[u8]) -> Result<T, RouterError> {
    let text = String::from_utf8_lossy(payload);
    let Some((_, body)) = text.split_once("\n\n") else {
        return Err(RouterError::Internal(
            "auth policy cell payload is missing body".to_owned(),
        ));
    };
    serde_json::from_str(body).map_err(|error| {
        RouterError::Internal(format!("auth policy cell payload is invalid: {error}"))
    })
}

fn policy_cell_id(key: &str) -> CellId {
    CellId(AUTH_POLICY_NAMESPACE | stable_hash(key))
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
    hash & 0x00ff_ffff_ffff_ffff
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn policy_cell_id_is_stable_and_namespaced() {
        let first = policy_cell_id("agent-a");
        let second = policy_cell_id("agent-a");

        assert_eq!(first, second);
        assert_eq!(first.0 & AUTH_POLICY_NAMESPACE, AUTH_POLICY_NAMESPACE);
    }

    #[test]
    fn token_fingerprint_does_not_reveal_token() {
        let fingerprint = token_fingerprint("super-secret-token");

        assert!(fingerprint.starts_with("fnv64:"));
        assert!(!fingerprint.contains("super-secret-token"));
    }
}
