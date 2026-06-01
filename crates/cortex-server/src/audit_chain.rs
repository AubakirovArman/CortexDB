use std::fs;
use std::io;
use std::path::Path;

pub(crate) const AUDIT_CHAIN_ID: &str = "cortexdb.audit.chain.v1";
pub(crate) const AUDIT_CHAIN_ZERO_HASH: &str = "0000000000000000";

const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;

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
    let mut hash = FNV_OFFSET;
    for (key, value) in fields {
        feed_hash(&mut hash, key, value);
    }
    format!("{hash:016x}")
}

pub(crate) fn is_hex_hash(value: &str) -> bool {
    value.len() == 16 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn feed_hash(hash: &mut u64, key: &str, value: &str) {
    for byte in key
        .as_bytes()
        .iter()
        .chain([0x1f].iter())
        .chain(value.as_bytes())
        .chain([0x1e].iter())
    {
        *hash ^= u64::from(*byte);
        *hash = hash.wrapping_mul(FNV_PRIME);
    }
}
