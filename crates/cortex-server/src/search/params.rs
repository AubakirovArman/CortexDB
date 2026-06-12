use cortex_engine::{AnnSearchPolicy, Database, HnswNoFallbackRolloutPolicy};

use crate::router::{query_param_decoded, query_param_opt};

use super::rerank::SearchRerankMode;

pub(super) fn parse_ann_policy(query: &str) -> Result<AnnSearchPolicy, String> {
    let default_policy = AnnSearchPolicy::default();
    let fallback = parse_optional_query_param(query, "fallback")?
        .map(|value| parse_bool("fallback", &value))
        .transpose()?
        .unwrap_or(default_policy.fallback);
    let fallback_scan_cap = parse_optional_query_param(query, "fallback_scan_cap")?
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|_| "fallback_scan_cap must be usize".to_owned())
        })
        .transpose()?;
    let min_recall_q16 = parse_optional_query_param(query, "min_recall")?
        .map(|value| parse_min_recall_q16(&value))
        .transpose()?
        .or(default_policy.min_recall_q16);

    let max_visited_candidates = parse_optional_query_param(query, "max_visited_candidates")?
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|_| "max_visited_candidates must be usize".to_owned())
        })
        .transpose()?;

    let require_slo = parse_optional_query_param(query, "require_slo")?
        .map(|value| parse_bool("require_slo", &value))
        .transpose()?
        .unwrap_or(default_policy.require_slo);

    Ok(AnnSearchPolicy {
        min_recall_q16,
        fallback,
        fallback_scan_cap,
        max_visited_candidates,
        require_slo,
    })
}

pub(super) fn parse_rerank_mode(query: &str) -> Result<SearchRerankMode, String> {
    let Some(value) = parse_optional_query_param(query, "rerank")? else {
        return Ok(SearchRerankMode::None);
    };
    match value.to_ascii_lowercase().as_str() {
        "" | "none" | "false" | "0" | "off" => Ok(SearchRerankMode::None),
        "weighted" | "true" | "1" | "on" => Ok(SearchRerankMode::Weighted),
        _ => Err("rerank must be none or weighted".to_owned()),
    }
}

pub(super) fn resolve_no_fallback_rollout_policy(
    db: &Database,
    query: &str,
) -> Result<Option<HnswNoFallbackRolloutPolicy>, String> {
    let use_profile = parse_optional_query_param(query, "no_fallback_profile")?
        .map(|value| parse_profile_selector(&value))
        .transpose()?
        .unwrap_or(false);
    let rollout = parse_optional_query_param(query, "no_fallback_rollout")?
        .map(|value| parse_bool("no_fallback_rollout", &value))
        .transpose()?;
    let min_recall = parse_optional_query_param(query, "no_fallback_min_recall")?
        .map(|value| parse_min_recall_q16(&value))
        .transpose()?;
    if use_profile && (rollout.is_some() || min_recall.is_some()) {
        return Err("no_fallback_profile cannot be combined with no_fallback_rollout or no_fallback_min_recall".to_owned());
    }
    if use_profile {
        return db
            .hnsw_no_fallback_rollout_policy()
            .map(Some)
            .ok_or_else(|| "no persisted HNSW no-fallback profile is configured".to_owned());
    }
    if rollout.is_none() && min_recall.is_none() {
        return Ok(None);
    }
    if rollout != Some(true) && min_recall.is_some() {
        return Err("no_fallback_min_recall requires no_fallback_rollout=true".to_owned());
    }
    let default_policy = HnswNoFallbackRolloutPolicy::default();
    let policy = HnswNoFallbackRolloutPolicy {
        rollout_enabled: rollout.unwrap_or(false),
        min_recall_q16: min_recall.unwrap_or(default_policy.min_recall_q16),
        require_upper_layers: default_policy.require_upper_layers,
    };
    Ok(Some(policy))
}

fn parse_profile_selector(value: &str) -> Result<bool, String> {
    match value.to_ascii_lowercase().as_str() {
        "active" | "true" | "1" | "yes" | "on" => Ok(true),
        "false" | "0" | "no" | "off" => Ok(false),
        _ => Err("no_fallback_profile must be active or true/false".to_owned()),
    }
}

fn parse_optional_query_param(query: &str, key: &str) -> Result<Option<String>, String> {
    if query_param_opt(query, key).is_some() {
        query_param_decoded(query, key).map(Some)
    } else {
        Ok(None)
    }
}

fn parse_bool(name: &str, value: &str) -> Result<bool, String> {
    match value.to_ascii_lowercase().as_str() {
        "true" | "1" | "yes" | "on" => Ok(true),
        "false" | "0" | "no" | "off" => Ok(false),
        _ => Err(format!("{name} must be true/false")),
    }
}

fn parse_min_recall_q16(value: &str) -> Result<u16, String> {
    let normalized = value.trim();
    let ratio = if normalized.ends_with('%') {
        let percent =
            parse_percent_without_unit(&normalized[..normalized.len().saturating_sub(1)])?;
        percent / 100.0
    } else {
        let number = normalized.parse::<f64>().map_err(|_| {
            "min_recall must be a decimal fraction, percentage, or integer q16".to_owned()
        })?;
        if number > 1.0 && number <= 100.0 {
            number / 100.0
        } else if number > 100.0 && number <= f64::from(u16::MAX) {
            number / f64::from(u16::MAX)
        } else {
            number
        }
    };

    if !(0.0..=1.0).contains(&ratio) {
        return Err("min_recall must be in [0.0, 1.0] or [0,100]%".to_owned());
    }
    Ok((ratio * f64::from(u16::MAX)) as u16)
}

fn parse_percent_without_unit(value: &str) -> Result<f64, String> {
    value
        .trim()
        .parse::<f64>()
        .map_err(|_| "min_recall must be percentage value".to_owned())
}

pub(super) fn parse_limit(value: &str) -> Result<usize, String> {
    value
        .parse::<usize>()
        .map(|limit| limit.max(1))
        .map_err(|_| "limit must be usize".to_owned())
}
