use cortex_engine::{Database, HnswNoFallbackRolloutPolicy};
use serde::Deserialize;

use crate::responses::{HnswNoFallbackProfileResponse, RouterError};

#[derive(Deserialize)]
struct HnswNoFallbackProfileRequest {
    rollout_enabled: bool,
    min_recall_q16: Option<u16>,
    require_upper_layers: Option<bool>,
}

pub(crate) fn handle_get(db: &Database) -> Result<String, RouterError> {
    Ok(serde_json::to_string(&profile_response(
        db.hnsw_no_fallback_rollout_policy(),
    ))?)
}

pub(crate) fn handle_put(db: &mut Database, body: &[u8]) -> Result<String, RouterError> {
    let request: HnswNoFallbackProfileRequest = serde_json::from_slice(body)?;
    let default_policy = HnswNoFallbackRolloutPolicy::enabled();
    let policy = HnswNoFallbackRolloutPolicy {
        rollout_enabled: request.rollout_enabled,
        min_recall_q16: request
            .min_recall_q16
            .unwrap_or(default_policy.min_recall_q16),
        require_upper_layers: request
            .require_upper_layers
            .unwrap_or(default_policy.require_upper_layers),
    };
    db.set_hnsw_no_fallback_rollout_policy(policy)?;
    Ok(serde_json::to_string(&profile_response(Some(policy)))?)
}

pub(crate) fn handle_delete(db: &mut Database) -> Result<String, RouterError> {
    db.clear_hnsw_no_fallback_rollout_policy()?;
    Ok(serde_json::to_string(&profile_response(None))?)
}

pub(crate) fn profile_response(
    policy: Option<HnswNoFallbackRolloutPolicy>,
) -> HnswNoFallbackProfileResponse {
    match policy {
        Some(policy) => HnswNoFallbackProfileResponse {
            configured: true,
            rollout_enabled: Some(policy.rollout_enabled),
            min_recall_q16: Some(policy.min_recall_q16),
            require_upper_layers: Some(policy.require_upper_layers),
        },
        None => HnswNoFallbackProfileResponse {
            configured: false,
            rollout_enabled: None,
            min_recall_q16: None,
            require_upper_layers: None,
        },
    }
}
