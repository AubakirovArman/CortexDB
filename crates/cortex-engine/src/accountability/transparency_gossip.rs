use std::collections::{BTreeMap, BTreeSet};

use serde_json::{json, Value};

mod types;

use super::receipt::hash_value;
use crate::error::{EngineError, EngineResult};
pub use types::{
    TransparencyGossipEvidence, TransparencyGossipExchange, TransparencyGossipPolicy,
    TRANSPARENCY_GOSSIP_EVIDENCE_SCHEMA, TRANSPARENCY_GOSSIP_EXCHANGE_SCHEMA,
    TRANSPARENCY_GOSSIP_HASH_DOMAIN,
};

pub fn build_transparency_gossip_evidence(
    policy: TransparencyGossipPolicy,
    mut exchanges: Vec<TransparencyGossipExchange>,
) -> EngineResult<TransparencyGossipEvidence> {
    validate_policy(&policy)?;
    exchanges.sort_by(|left, right| {
        left.sender_monitor_id
            .cmp(&right.sender_monitor_id)
            .then(left.receiver_monitor_id.cmp(&right.receiver_monitor_id))
            .then(left.exchange_unix_seconds.cmp(&right.exchange_unix_seconds))
    });
    let first = exchanges
        .first()
        .ok_or_else(|| gossip_invariant("transparency gossip requires exchanges"))?;
    let monitor_count = monitor_identities(&exchanges)?.len() as u64;
    let mut evidence = TransparencyGossipEvidence {
        schema_version: TRANSPARENCY_GOSSIP_EVIDENCE_SCHEMA.to_owned(),
        service_id: policy.service_id,
        service_url: policy.service_url,
        window_start_unix_seconds: policy.window_start_unix_seconds,
        window_end_unix_seconds: policy.window_end_unix_seconds,
        required_monitor_count: policy.required_monitor_count,
        required_fanout: policy.required_fanout,
        max_exchange_age_seconds: policy.max_exchange_age_seconds,
        monitor_count,
        exchange_count: exchanges.len() as u64,
        log_record_count: first.log_record_count,
        log_head_hash: first.log_head_hash.clone(),
        merkle_root_hash: first.merkle_root_hash.clone(),
        exchanges,
        gossip_hash: String::new(),
    };
    evidence.gossip_hash = transparency_gossip_hash(&evidence);
    verify_transparency_gossip_evidence(&evidence)?;
    Ok(evidence)
}

pub fn verify_transparency_gossip_evidence(
    evidence: &TransparencyGossipEvidence,
) -> EngineResult<()> {
    verify_evidence_shape(evidence)?;
    let monitors = monitor_identities(&evidence.exchanges)?;
    if monitors.len() as u64 != evidence.monitor_count {
        return fail("transparency gossip monitor count mismatch");
    }
    if evidence.monitor_count < evidence.required_monitor_count {
        return fail("transparency gossip monitor quorum not met");
    }
    verify_fanout(evidence, monitors.keys())?;
    if evidence.gossip_hash != transparency_gossip_hash(evidence) {
        return fail("transparency gossip hash mismatch");
    }
    Ok(())
}

fn verify_fanout<'a>(
    evidence: &TransparencyGossipEvidence,
    monitors: impl Iterator<Item = &'a String>,
) -> EngineResult<()> {
    let mut fanout: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    for monitor in monitors {
        fanout.insert(monitor.as_str(), BTreeSet::new());
    }
    for exchange in &evidence.exchanges {
        verify_exchange(exchange, evidence)?;
        fanout
            .entry(exchange.sender_monitor_id.as_str())
            .or_default()
            .insert(exchange.receiver_monitor_id.as_str());
    }
    for receivers in fanout.values() {
        if receivers.len() < evidence.required_fanout as usize {
            return fail("transparency gossip fanout not met");
        }
    }
    Ok(())
}

fn validate_policy(policy: &TransparencyGossipPolicy) -> EngineResult<()> {
    required_label("service_id", &policy.service_id)?;
    require_https_url("service_url", &policy.service_url)?;
    if policy.window_start_unix_seconds > policy.window_end_unix_seconds {
        return fail("transparency gossip window is inverted");
    }
    if policy.required_monitor_count < 3 || policy.required_fanout < 2 {
        return fail("transparency gossip requires at least three monitors and fanout two");
    }
    if policy.max_exchange_age_seconds == 0 {
        return fail("transparency gossip policy requires non-zero freshness");
    }
    Ok(())
}

fn verify_evidence_shape(evidence: &TransparencyGossipEvidence) -> EngineResult<()> {
    if evidence.schema_version != TRANSPARENCY_GOSSIP_EVIDENCE_SCHEMA {
        return fail("invalid transparency gossip evidence schema");
    }
    validate_policy(&TransparencyGossipPolicy {
        service_id: evidence.service_id.clone(),
        service_url: evidence.service_url.clone(),
        window_start_unix_seconds: evidence.window_start_unix_seconds,
        window_end_unix_seconds: evidence.window_end_unix_seconds,
        required_monitor_count: evidence.required_monitor_count,
        required_fanout: evidence.required_fanout,
        max_exchange_age_seconds: evidence.max_exchange_age_seconds,
    })?;
    if evidence.exchange_count != evidence.exchanges.len() as u64 {
        return fail("transparency gossip exchange count mismatch");
    }
    require_hash("log_head_hash", &evidence.log_head_hash)?;
    require_hash("merkle_root_hash", &evidence.merkle_root_hash)?;
    require_hash("gossip_hash", &evidence.gossip_hash)?;
    Ok(())
}

fn verify_exchange(
    exchange: &TransparencyGossipExchange,
    evidence: &TransparencyGossipEvidence,
) -> EngineResult<()> {
    if exchange.schema_version != TRANSPARENCY_GOSSIP_EXCHANGE_SCHEMA {
        return fail("invalid transparency gossip exchange schema");
    }
    required_label("sender_monitor_id", &exchange.sender_monitor_id)?;
    required_label("receiver_monitor_id", &exchange.receiver_monitor_id)?;
    require_https_url("sender_monitor_url", &exchange.sender_monitor_url)?;
    require_https_url("receiver_monitor_url", &exchange.receiver_monitor_url)?;
    require_https_url("service_url", &exchange.service_url)?;
    if exchange.service_url != evidence.service_url {
        return fail("transparency gossip service url mismatch");
    }
    if exchange.sender_monitor_id == exchange.receiver_monitor_id {
        return fail("transparency gossip self exchange is invalid");
    }
    if exchange.gossip_status != "delivered"
        || !(200..=299).contains(&exchange.response_http_status)
    {
        return fail("transparency gossip exchange was not delivered");
    }
    verify_exchange_window(exchange, evidence)?;
    verify_exchange_head(exchange, evidence)
}

fn verify_exchange_window(
    exchange: &TransparencyGossipExchange,
    evidence: &TransparencyGossipEvidence,
) -> EngineResult<()> {
    if exchange.exchange_unix_seconds < evidence.window_start_unix_seconds
        || exchange.exchange_unix_seconds > evidence.window_end_unix_seconds
    {
        return fail("transparency gossip exchange outside window");
    }
    if exchange
        .exchange_unix_seconds
        .saturating_add(evidence.max_exchange_age_seconds)
        < evidence.window_end_unix_seconds
    {
        return fail("stale transparency gossip exchange");
    }
    Ok(())
}

fn verify_exchange_head(
    exchange: &TransparencyGossipExchange,
    evidence: &TransparencyGossipEvidence,
) -> EngineResult<()> {
    if exchange.log_record_count != evidence.log_record_count {
        return fail("split transparency gossip log count");
    }
    require_hash("log_head_hash", &exchange.log_head_hash)?;
    require_hash("merkle_root_hash", &exchange.merkle_root_hash)?;
    if exchange.log_head_hash != evidence.log_head_hash {
        return fail("split transparency gossip log head");
    }
    if exchange.merkle_root_hash != evidence.merkle_root_hash {
        return fail("split transparency gossip merkle root");
    }
    Ok(())
}

fn monitor_identities(
    exchanges: &[TransparencyGossipExchange],
) -> EngineResult<BTreeMap<String, String>> {
    let mut by_id = BTreeMap::new();
    let mut by_url = BTreeMap::new();
    for exchange in exchanges {
        remember_monitor(
            &mut by_id,
            &mut by_url,
            &exchange.sender_monitor_id,
            &exchange.sender_monitor_url,
        )?;
        remember_monitor(
            &mut by_id,
            &mut by_url,
            &exchange.receiver_monitor_id,
            &exchange.receiver_monitor_url,
        )?;
    }
    Ok(by_id)
}

fn remember_monitor(
    by_id: &mut BTreeMap<String, String>,
    by_url: &mut BTreeMap<String, String>,
    monitor_id: &str,
    monitor_url: &str,
) -> EngineResult<()> {
    if let Some(existing_url) = by_id.insert(monitor_id.to_owned(), monitor_url.to_owned()) {
        if existing_url != monitor_url {
            return fail("conflicting transparency gossip monitor url");
        }
    }
    if let Some(existing_id) = by_url.insert(monitor_url.to_owned(), monitor_id.to_owned()) {
        if existing_id != monitor_id {
            return fail("duplicate transparency gossip monitor url");
        }
    }
    Ok(())
}

fn transparency_gossip_hash(evidence: &TransparencyGossipEvidence) -> String {
    hash_value(
        TRANSPARENCY_GOSSIP_HASH_DOMAIN,
        &transparency_gossip_body(evidence),
    )
}

fn transparency_gossip_body(evidence: &TransparencyGossipEvidence) -> Value {
    json!({
        "schema_version": evidence.schema_version,
        "service_id": evidence.service_id,
        "service_url": evidence.service_url,
        "window_start_unix_seconds": evidence.window_start_unix_seconds,
        "window_end_unix_seconds": evidence.window_end_unix_seconds,
        "required_monitor_count": evidence.required_monitor_count,
        "required_fanout": evidence.required_fanout,
        "max_exchange_age_seconds": evidence.max_exchange_age_seconds,
        "monitor_count": evidence.monitor_count,
        "exchange_count": evidence.exchange_count,
        "log_record_count": evidence.log_record_count,
        "log_head_hash": evidence.log_head_hash,
        "merkle_root_hash": evidence.merkle_root_hash,
        "exchanges": evidence.exchanges,
    })
}

fn require_hash(name: &str, value: &str) -> EngineResult<()> {
    if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return fail(format!("{name} must be 64 hex chars"));
    }
    Ok(())
}

fn require_https_url(name: &str, value: &str) -> EngineResult<()> {
    let trimmed = value.trim();
    if trimmed.is_empty() || !trimmed.starts_with("https://") {
        return fail(format!("{name} must be a non-empty https URL"));
    }
    Ok(())
}

fn required_label(name: &str, value: &str) -> EngineResult<()> {
    if value.trim().is_empty() {
        return fail(format!("{name} is required"));
    }
    Ok(())
}

fn fail<T>(message: impl Into<String>) -> EngineResult<T> {
    Err(gossip_invariant(message))
}

fn gossip_invariant(message: impl Into<String>) -> EngineError {
    EngineError::StorageInvariant(message.into())
}
