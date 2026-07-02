use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use super::receipt::hash_value;
use crate::error::{EngineError, EngineResult};

pub const TRANSPARENCY_AVAILABILITY_OBSERVATION_SCHEMA: &str =
    "cortexdb.transparency.availability.observation.v1";
pub const TRANSPARENCY_AVAILABILITY_EVIDENCE_SCHEMA: &str =
    "cortexdb.transparency.availability.evidence.v1";
pub const TRANSPARENCY_AVAILABILITY_HASH_DOMAIN: &str =
    "cortexdb.transparency.availability_hash.v1";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransparencyAvailabilityPolicy {
    pub service_id: String,
    pub service_url: String,
    pub window_start_unix_seconds: u64,
    pub window_end_unix_seconds: u64,
    pub required_monitor_count: u64,
    pub required_monitor_uptime_seconds: u64,
    pub max_observation_age_seconds: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransparencyAvailabilityObservation {
    pub schema_version: String,
    pub monitor_id: String,
    pub monitor_url: String,
    pub service_url: String,
    pub observed_unix_seconds: u64,
    pub response_http_status: u16,
    pub monitor_uptime_seconds: u64,
    pub log_record_count: u64,
    pub log_head_hash: String,
    pub merkle_root_hash: String,
    pub availability_status: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransparencyAvailabilityEvidence {
    pub schema_version: String,
    pub service_id: String,
    pub service_url: String,
    pub window_start_unix_seconds: u64,
    pub window_end_unix_seconds: u64,
    pub required_monitor_count: u64,
    pub required_monitor_uptime_seconds: u64,
    pub max_observation_age_seconds: u64,
    pub log_record_count: u64,
    pub log_head_hash: String,
    pub merkle_root_hash: String,
    pub observations: Vec<TransparencyAvailabilityObservation>,
    pub availability_hash: String,
}

pub fn build_transparency_availability_evidence(
    policy: TransparencyAvailabilityPolicy,
    mut observations: Vec<TransparencyAvailabilityObservation>,
) -> EngineResult<TransparencyAvailabilityEvidence> {
    validate_policy(&policy)?;
    observations.sort_by(|left, right| {
        left.monitor_id
            .cmp(&right.monitor_id)
            .then(left.monitor_url.cmp(&right.monitor_url))
    });
    let first = observations
        .first()
        .ok_or_else(|| availability_invariant("transparency availability requires observations"))?;

    let mut evidence = TransparencyAvailabilityEvidence {
        schema_version: TRANSPARENCY_AVAILABILITY_EVIDENCE_SCHEMA.to_owned(),
        service_id: policy.service_id,
        service_url: policy.service_url,
        window_start_unix_seconds: policy.window_start_unix_seconds,
        window_end_unix_seconds: policy.window_end_unix_seconds,
        required_monitor_count: policy.required_monitor_count,
        required_monitor_uptime_seconds: policy.required_monitor_uptime_seconds,
        max_observation_age_seconds: policy.max_observation_age_seconds,
        log_record_count: first.log_record_count,
        log_head_hash: first.log_head_hash.clone(),
        merkle_root_hash: first.merkle_root_hash.clone(),
        observations,
        availability_hash: String::new(),
    };
    evidence.availability_hash = transparency_availability_hash(&evidence);
    verify_transparency_availability_evidence(&evidence)?;
    Ok(evidence)
}

pub fn verify_transparency_availability_evidence(
    evidence: &TransparencyAvailabilityEvidence,
) -> EngineResult<()> {
    verify_evidence_shape(evidence)?;

    let required_count = usize::try_from(evidence.required_monitor_count).map_err(|_| {
        availability_invariant("transparency availability monitor count exceeds platform size")
    })?;
    if evidence.observations.len() < required_count {
        return fail("transparency availability monitor quorum not met");
    }

    let mut monitor_ids = HashSet::new();
    let mut monitor_urls = HashSet::new();
    for observation in &evidence.observations {
        verify_observation(observation, evidence)?;
        if !monitor_ids.insert(observation.monitor_id.as_str()) {
            return Err(availability_invariant("duplicate transparency monitor id"));
        }
        if !monitor_urls.insert(observation.monitor_url.as_str()) {
            return Err(availability_invariant("duplicate transparency monitor url"));
        }
    }

    if evidence.availability_hash != transparency_availability_hash(evidence) {
        return fail("transparency availability hash mismatch");
    }
    Ok(())
}

fn validate_policy(policy: &TransparencyAvailabilityPolicy) -> EngineResult<()> {
    required_label("service_id", &policy.service_id)?;
    require_https_url("service_url", &policy.service_url)?;
    if policy.window_start_unix_seconds > policy.window_end_unix_seconds {
        return fail("transparency availability window is inverted");
    }
    if policy.required_monitor_count < 2 {
        return fail("transparency availability requires at least two monitors");
    }
    if policy.required_monitor_uptime_seconds == 0 || policy.max_observation_age_seconds == 0 {
        return fail("transparency availability policy requires non-zero uptime and freshness");
    }
    Ok(())
}

fn verify_evidence_shape(evidence: &TransparencyAvailabilityEvidence) -> EngineResult<()> {
    if evidence.schema_version != TRANSPARENCY_AVAILABILITY_EVIDENCE_SCHEMA {
        return fail("invalid transparency availability evidence schema");
    }
    validate_policy(&TransparencyAvailabilityPolicy {
        service_id: evidence.service_id.clone(),
        service_url: evidence.service_url.clone(),
        window_start_unix_seconds: evidence.window_start_unix_seconds,
        window_end_unix_seconds: evidence.window_end_unix_seconds,
        required_monitor_count: evidence.required_monitor_count,
        required_monitor_uptime_seconds: evidence.required_monitor_uptime_seconds,
        max_observation_age_seconds: evidence.max_observation_age_seconds,
    })?;
    if evidence.log_record_count == 0 {
        return fail("transparency availability requires a non-empty published log");
    }
    require_hash("log_head_hash", &evidence.log_head_hash)?;
    require_hash("merkle_root_hash", &evidence.merkle_root_hash)?;
    require_hash("availability_hash", &evidence.availability_hash)?;
    Ok(())
}

fn verify_observation(
    observation: &TransparencyAvailabilityObservation,
    evidence: &TransparencyAvailabilityEvidence,
) -> EngineResult<()> {
    if observation.schema_version != TRANSPARENCY_AVAILABILITY_OBSERVATION_SCHEMA {
        return fail("invalid transparency availability observation schema");
    }
    required_label("monitor_id", &observation.monitor_id)?;
    require_https_url("monitor_url", &observation.monitor_url)?;
    require_https_url("service_url", &observation.service_url)?;
    if observation.service_url != evidence.service_url {
        return fail("transparency availability service url mismatch");
    }
    if observation.availability_status != "available"
        || !(200..=299).contains(&observation.response_http_status)
    {
        return fail("transparency service was not available to monitor");
    }
    verify_observation_window(observation, evidence)?;
    verify_observation_head(observation, evidence)?;
    Ok(())
}

fn verify_observation_window(
    observation: &TransparencyAvailabilityObservation,
    evidence: &TransparencyAvailabilityEvidence,
) -> EngineResult<()> {
    if observation.observed_unix_seconds < evidence.window_start_unix_seconds
        || observation.observed_unix_seconds > evidence.window_end_unix_seconds
    {
        return fail("transparency availability observation outside window");
    }
    if observation
        .observed_unix_seconds
        .saturating_add(evidence.max_observation_age_seconds)
        < evidence.window_end_unix_seconds
    {
        return fail("stale transparency availability observation");
    }
    if observation.monitor_uptime_seconds < evidence.required_monitor_uptime_seconds {
        return fail("transparency monitor uptime below policy");
    }
    Ok(())
}

fn verify_observation_head(
    observation: &TransparencyAvailabilityObservation,
    evidence: &TransparencyAvailabilityEvidence,
) -> EngineResult<()> {
    if observation.log_record_count != evidence.log_record_count {
        return fail("split transparency availability log count");
    }
    require_hash("log_head_hash", &observation.log_head_hash)?;
    require_hash("merkle_root_hash", &observation.merkle_root_hash)?;
    if observation.log_head_hash != evidence.log_head_hash {
        return fail("split transparency availability log head");
    }
    if observation.merkle_root_hash != evidence.merkle_root_hash {
        return fail("split transparency availability merkle root");
    }
    Ok(())
}

fn transparency_availability_hash(evidence: &TransparencyAvailabilityEvidence) -> String {
    hash_value(
        TRANSPARENCY_AVAILABILITY_HASH_DOMAIN,
        &transparency_availability_body(evidence),
    )
}

fn transparency_availability_body(evidence: &TransparencyAvailabilityEvidence) -> Value {
    json!({
        "schema_version": evidence.schema_version,
        "service_id": evidence.service_id,
        "service_url": evidence.service_url,
        "window_start_unix_seconds": evidence.window_start_unix_seconds,
        "window_end_unix_seconds": evidence.window_end_unix_seconds,
        "required_monitor_count": evidence.required_monitor_count,
        "required_monitor_uptime_seconds": evidence.required_monitor_uptime_seconds,
        "max_observation_age_seconds": evidence.max_observation_age_seconds,
        "log_record_count": evidence.log_record_count,
        "log_head_hash": evidence.log_head_hash,
        "merkle_root_hash": evidence.merkle_root_hash,
        "observations": evidence.observations,
    })
}

fn require_hash(name: &str, value: &str) -> EngineResult<()> {
    if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(availability_invariant(format!(
            "{name} must be 64 hex chars"
        )));
    }
    Ok(())
}

fn require_https_url(name: &str, value: &str) -> EngineResult<()> {
    let trimmed = value.trim();
    if trimmed.is_empty() || !trimmed.starts_with("https://") {
        return Err(availability_invariant(format!(
            "{name} must be a non-empty https URL"
        )));
    }
    Ok(())
}

fn required_label(name: &str, value: &str) -> EngineResult<()> {
    if value.trim().is_empty() {
        return Err(availability_invariant(format!("{name} is required")));
    }
    Ok(())
}

fn fail<T>(message: impl Into<String>) -> EngineResult<T> {
    Err(availability_invariant(message))
}

fn availability_invariant(message: impl Into<String>) -> EngineError {
    EngineError::StorageInvariant(message.into())
}
