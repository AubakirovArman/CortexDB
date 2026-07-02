use std::fs;
use std::io;
use std::path::Path;

pub(crate) use cortex_crypto::audit_chain::{AUDIT_CHAIN_ID, AUDIT_CHAIN_ZERO_HASH};

#[derive(serde::Deserialize)]
struct AuditChainTail {
    sequence: Option<u64>,
    event_hash: Option<String>,
}

pub(crate) fn tail(path: &Path) -> io::Result<(u64, String)> {
    let Ok(raw) = fs::read_to_string(path) else {
        return Ok((1, AUDIT_CHAIN_ZERO_HASH.to_owned()));
    };
    let Some(line) = raw.lines().rev().find(|line| !line.trim().is_empty()) else {
        return Ok((1, AUDIT_CHAIN_ZERO_HASH.to_owned()));
    };
    let tail = serde_json::from_str::<AuditChainTail>(line)
        .map_err(|error| io::Error::other(format!("audit chain tail is invalid JSON: {error}")))?;
    if tail.sequence.is_none() && tail.event_hash.is_none() {
        return Ok((1, AUDIT_CHAIN_ZERO_HASH.to_owned()));
    }
    let sequence = tail
        .sequence
        .ok_or_else(|| io::Error::other("audit chain tail is missing sequence"))?;
    let event_hash = tail
        .event_hash
        .filter(|hash| is_hex_hash(hash))
        .ok_or_else(|| io::Error::other("audit chain tail has invalid event_hash"))?;
    let next_sequence = sequence
        .checked_add(1)
        .ok_or_else(|| io::Error::other("audit chain sequence overflow"))?;
    Ok((next_sequence, event_hash))
}

pub(crate) fn event_hash(fields: &[(&str, String)]) -> String {
    cortex_crypto::audit_chain::event_hash(fields)
}

pub(crate) fn event_mac(key: &cortex_crypto::MacKey, fields: &[(&str, String)]) -> String {
    cortex_crypto::audit_chain::event_mac(key, fields)
}

pub(crate) fn is_hex_hash(value: &str) -> bool {
    cortex_crypto::audit_chain::is_hex_hash(value)
}
