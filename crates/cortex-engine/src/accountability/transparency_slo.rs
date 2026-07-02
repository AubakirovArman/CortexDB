use std::collections::BTreeSet;

use serde_json::{json, Value};

mod types;
mod validation;

use super::receipt::hash_value;
use crate::error::EngineResult;
pub use types::{
    TransparencySloEvidence, TransparencySloPolicy, TransparencySloWindow,
    TRANSPARENCY_SLO_EVIDENCE_SCHEMA, TRANSPARENCY_SLO_HASH_DOMAIN, TRANSPARENCY_SLO_WINDOW_SCHEMA,
};
use validation::{
    fail, require_hash, require_https_url, required_label, slo_invariant, usize_to_u64,
};

pub fn build_transparency_slo_evidence(
    policy: TransparencySloPolicy,
    mut windows: Vec<TransparencySloWindow>,
) -> EngineResult<TransparencySloEvidence> {
    validate_policy(&policy)?;
    windows.sort_by(|left, right| {
        left.window_start_unix_seconds
            .cmp(&right.window_start_unix_seconds)
            .then(
                left.window_end_unix_seconds
                    .cmp(&right.window_end_unix_seconds),
            )
            .then(left.window_id.cmp(&right.window_id))
    });
    let last = windows
        .last()
        .ok_or_else(|| slo_invariant("transparency slo requires windows"))?;
    let window_count = usize_to_u64(windows.len(), "transparency slo window count overflow")?;
    let available_window_count = available_window_count(&windows)?;

    let mut evidence = TransparencySloEvidence {
        schema_version: TRANSPARENCY_SLO_EVIDENCE_SCHEMA.to_owned(),
        service_id: policy.service_id,
        service_url: policy.service_url,
        period_start_unix_seconds: policy.period_start_unix_seconds,
        period_end_unix_seconds: policy.period_end_unix_seconds,
        required_window_count: policy.required_window_count,
        min_available_window_percentage: policy.min_available_window_percentage,
        max_window_gap_seconds: policy.max_window_gap_seconds,
        required_monitor_count: policy.required_monitor_count,
        required_gossip_fanout: policy.required_gossip_fanout,
        window_count,
        available_window_count,
        log_record_count: last.log_record_count,
        log_head_hash: last.log_head_hash.clone(),
        merkle_root_hash: last.merkle_root_hash.clone(),
        windows,
        slo_hash: String::new(),
    };
    evidence.slo_hash = transparency_slo_hash(&evidence);
    verify_transparency_slo_evidence(&evidence)?;
    Ok(evidence)
}

pub fn verify_transparency_slo_evidence(evidence: &TransparencySloEvidence) -> EngineResult<()> {
    verify_evidence_shape(evidence)?;
    let required_count = usize::try_from(evidence.required_window_count)
        .map_err(|_| slo_invariant("transparency slo required window count exceeds platform"))?;
    if evidence.windows.len() < required_count {
        return fail("transparency slo window quorum not met");
    }

    verify_windows(evidence)?;
    if !availability_target_met(
        evidence.available_window_count,
        evidence.window_count,
        evidence.min_available_window_percentage,
    ) {
        return fail("transparency slo availability target not met");
    }
    if evidence.slo_hash != transparency_slo_hash(evidence) {
        return fail("transparency slo hash mismatch");
    }
    Ok(())
}

fn verify_windows(evidence: &TransparencySloEvidence) -> EngineResult<()> {
    let mut seen_ids = BTreeSet::new();
    let mut previous: Option<&TransparencySloWindow> = None;
    let mut counted_available = 0_u64;

    for window in &evidence.windows {
        verify_window(window, evidence)?;
        if !seen_ids.insert(window.window_id.as_str()) {
            return fail("duplicate transparency slo window id");
        }
        if window.availability_status == "available" {
            counted_available += 1;
        }
        if let Some(previous_window) = previous {
            verify_window_order(previous_window, window, evidence)?;
        } else if window.window_start_unix_seconds != evidence.period_start_unix_seconds {
            return fail("transparency slo period start gap");
        }
        previous = Some(window);
    }

    let last = previous.ok_or_else(|| slo_invariant("transparency slo requires windows"))?;
    if last.window_end_unix_seconds != evidence.period_end_unix_seconds {
        return fail("transparency slo period end gap");
    }
    if counted_available != evidence.available_window_count {
        return fail("transparency slo available window count mismatch");
    }
    if evidence.log_record_count != last.log_record_count
        || evidence.log_head_hash != last.log_head_hash
        || evidence.merkle_root_hash != last.merkle_root_hash
    {
        return fail("transparency slo final log head mismatch");
    }
    Ok(())
}

fn validate_policy(policy: &TransparencySloPolicy) -> EngineResult<()> {
    required_label("service_id", &policy.service_id)?;
    require_https_url("service_url", &policy.service_url)?;
    if policy.period_start_unix_seconds > policy.period_end_unix_seconds {
        return fail("transparency slo period is inverted");
    }
    if policy.required_window_count < 2 {
        return fail("transparency slo requires at least two windows");
    }
    if policy.min_available_window_percentage == 0 || policy.min_available_window_percentage > 100 {
        return fail("transparency slo availability percentage must be 1..=100");
    }
    if policy.required_monitor_count < 2 || policy.required_gossip_fanout < 2 {
        return fail("transparency slo requires monitor quorum and gossip fanout");
    }
    Ok(())
}

fn verify_evidence_shape(evidence: &TransparencySloEvidence) -> EngineResult<()> {
    if evidence.schema_version != TRANSPARENCY_SLO_EVIDENCE_SCHEMA {
        return fail("invalid transparency slo evidence schema");
    }
    validate_policy(&TransparencySloPolicy {
        service_id: evidence.service_id.clone(),
        service_url: evidence.service_url.clone(),
        period_start_unix_seconds: evidence.period_start_unix_seconds,
        period_end_unix_seconds: evidence.period_end_unix_seconds,
        required_window_count: evidence.required_window_count,
        min_available_window_percentage: evidence.min_available_window_percentage,
        max_window_gap_seconds: evidence.max_window_gap_seconds,
        required_monitor_count: evidence.required_monitor_count,
        required_gossip_fanout: evidence.required_gossip_fanout,
    })?;
    if evidence.window_count != usize_to_u64(evidence.windows.len(), "window count overflow")? {
        return fail("transparency slo window count mismatch");
    }
    if evidence.log_record_count == 0 {
        return fail("transparency slo requires a non-empty published log");
    }
    require_hash("log_head_hash", &evidence.log_head_hash)?;
    require_hash("merkle_root_hash", &evidence.merkle_root_hash)?;
    require_hash("slo_hash", &evidence.slo_hash)?;
    Ok(())
}

fn verify_window(
    window: &TransparencySloWindow,
    evidence: &TransparencySloEvidence,
) -> EngineResult<()> {
    if window.schema_version != TRANSPARENCY_SLO_WINDOW_SCHEMA {
        return fail("invalid transparency slo window schema");
    }
    required_label("window_id", &window.window_id)?;
    require_https_url("service_url", &window.service_url)?;
    if window.service_url != evidence.service_url {
        return fail("transparency slo service url mismatch");
    }
    if window.window_start_unix_seconds > window.window_end_unix_seconds {
        return fail("transparency slo window is inverted");
    }
    if window.window_start_unix_seconds < evidence.period_start_unix_seconds
        || window.window_end_unix_seconds > evidence.period_end_unix_seconds
    {
        return fail("transparency slo window outside period");
    }
    verify_window_status(window, evidence)
}

fn verify_window_status(
    window: &TransparencySloWindow,
    evidence: &TransparencySloEvidence,
) -> EngineResult<()> {
    if !matches!(
        window.availability_status.as_str(),
        "available" | "unavailable"
    ) {
        return fail("invalid transparency slo availability status");
    }
    if window.monitor_count < evidence.required_monitor_count {
        return fail("transparency slo monitor quorum not met");
    }
    if window.gossip_fanout < evidence.required_gossip_fanout {
        return fail("transparency slo gossip fanout not met");
    }
    if window.consistency_status != "append_only" {
        return fail("transparency slo consistency not append-only");
    }
    if window.log_record_count == 0 {
        return fail("transparency slo window requires a non-empty log");
    }
    require_hash("log_head_hash", &window.log_head_hash)?;
    require_hash("merkle_root_hash", &window.merkle_root_hash)?;
    Ok(())
}

fn verify_window_order(
    previous: &TransparencySloWindow,
    current: &TransparencySloWindow,
    evidence: &TransparencySloEvidence,
) -> EngineResult<()> {
    if current.window_start_unix_seconds <= previous.window_start_unix_seconds {
        return fail("transparency slo windows are not ordered");
    }
    let allowed_next_start = previous
        .window_end_unix_seconds
        .saturating_add(1)
        .saturating_add(evidence.max_window_gap_seconds);
    if current.window_start_unix_seconds > allowed_next_start {
        return fail("transparency slo window gap");
    }
    verify_log_progress(previous, current)
}

fn verify_log_progress(
    previous: &TransparencySloWindow,
    current: &TransparencySloWindow,
) -> EngineResult<()> {
    if current.log_record_count < previous.log_record_count {
        return fail("transparency slo log count regressed");
    }
    if current.log_record_count == previous.log_record_count {
        if current.log_head_hash != previous.log_head_hash {
            return fail("split transparency slo log head");
        }
        if current.merkle_root_hash != previous.merkle_root_hash {
            return fail("split transparency slo merkle root");
        }
    }
    Ok(())
}

fn available_window_count(windows: &[TransparencySloWindow]) -> EngineResult<u64> {
    usize_to_u64(
        windows
            .iter()
            .filter(|window| window.availability_status == "available")
            .count(),
        "transparency slo available window count overflow",
    )
}

fn availability_target_met(available_windows: u64, total_windows: u64, target: u64) -> bool {
    available_windows.saturating_mul(100) >= total_windows.saturating_mul(target)
}

fn transparency_slo_hash(evidence: &TransparencySloEvidence) -> String {
    hash_value(
        TRANSPARENCY_SLO_HASH_DOMAIN,
        &transparency_slo_body(evidence),
    )
}

fn transparency_slo_body(evidence: &TransparencySloEvidence) -> Value {
    json!({
        "schema_version": evidence.schema_version,
        "service_id": evidence.service_id,
        "service_url": evidence.service_url,
        "period_start_unix_seconds": evidence.period_start_unix_seconds,
        "period_end_unix_seconds": evidence.period_end_unix_seconds,
        "required_window_count": evidence.required_window_count,
        "min_available_window_percentage": evidence.min_available_window_percentage,
        "max_window_gap_seconds": evidence.max_window_gap_seconds,
        "required_monitor_count": evidence.required_monitor_count,
        "required_gossip_fanout": evidence.required_gossip_fanout,
        "window_count": evidence.window_count,
        "available_window_count": evidence.available_window_count,
        "log_record_count": evidence.log_record_count,
        "log_head_hash": evidence.log_head_hash,
        "merkle_root_hash": evidence.merkle_root_hash,
        "windows": evidence.windows,
    })
}
