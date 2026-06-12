use std::sync::atomic::Ordering;

use crate::responses::RouterError;
use crate::state::AppState;
use crate::{auth, rate_limit::PrincipalQueuePermit};

pub(crate) fn request_allowed_by_rate_limit(state: &AppState) -> bool {
    let Some(rate_limit) = &state.rate_limit else {
        return true;
    };
    rate_limit.allow()
}

pub(crate) fn request_allowed_by_principal_quota(
    state: &AppState,
    decision: &auth::AuthDecision,
) -> bool {
    let Some(limit) = decision.request_quota_per_minute else {
        return true;
    };
    let Some(quota_key) = decision.quota_key.as_deref() else {
        return true;
    };
    let allowed = state.principal_rate_limits.allow(quota_key, limit);
    if allowed {
        state
            .principal_quota_requests_allowed
            .fetch_add(1, Ordering::Relaxed);
    } else {
        state
            .principal_quota_requests_rejected
            .fetch_add(1, Ordering::Relaxed);
    }
    allowed
}

pub(crate) fn request_body_allowed_by_principal_quota(
    state: &AppState,
    decision: &auth::AuthDecision,
    body_len: usize,
) -> bool {
    let Some(limit) = decision.body_quota_bytes_per_minute else {
        return true;
    };
    let Some(quota_key) = decision.quota_key.as_deref() else {
        return true;
    };
    let allowed = state
        .principal_rate_limits
        .allow_body_bytes(quota_key, limit, body_len as u64);
    if allowed {
        state
            .principal_quota_body_bytes_allowed
            .fetch_add(body_len as u64, Ordering::Relaxed);
    } else {
        state
            .principal_quota_body_bytes_rejected
            .fetch_add(body_len as u64, Ordering::Relaxed);
    }
    allowed
}

pub(crate) fn acquire_principal_queue_permit(
    state: &AppState,
    decision: &auth::AuthDecision,
) -> Result<Option<PrincipalQueuePermit>, RouterError> {
    let Some(limit) = decision.queue_quota else {
        return Ok(None);
    };
    let Some(quota_key) = decision.quota_key.as_deref() else {
        return Ok(None);
    };
    let permit = state
        .principal_rate_limits
        .acquire_queue(quota_key, limit)
        .ok_or_else(|| {
            state
                .principal_quota_queue_rejected
                .fetch_add(1, Ordering::Relaxed);
            RouterError::RateLimited
        })?;
    state
        .principal_quota_queue_acquired
        .fetch_add(1, Ordering::Relaxed);
    Ok(Some(permit))
}
