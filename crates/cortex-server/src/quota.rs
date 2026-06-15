use std::sync::atomic::Ordering;

use cortex_api_types::core::{WriteBatchOperationRequest, WriteBatchRequest};

use crate::actor::DatabaseActor;
use crate::responses::RouterError;
use crate::state::AppState;
use crate::{
    auth,
    rate_limit::{PrincipalQueuePermit, TenantQueuePermit},
};

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
            RouterError::QuotaExceeded("principal queue quota exceeded".to_owned())
        })?;
    state
        .principal_quota_queue_acquired
        .fetch_add(1, Ordering::Relaxed);
    Ok(Some(permit))
}

pub(crate) fn acquire_tenant_queue_permit(
    state: &AppState,
    tenant: &str,
) -> Result<Option<TenantQueuePermit>, RouterError> {
    let Some(limit) = state.options.tenant_queue_quota else {
        return Ok(None);
    };
    state
        .tenant_queue_limits
        .acquire_queue(tenant, limit)
        .ok_or_else(|| RouterError::QuotaExceeded("tenant queue quota exceeded".to_owned()))
        .map(Some)
}

pub(crate) fn enforce_tenant_storage_quota(
    state: &AppState,
    db: &DatabaseActor,
    method: &str,
    target: &str,
    body: &[u8],
) -> Result<(), RouterError> {
    if state.options.tenant_max_cells.is_none() && state.options.tenant_max_memory_bytes.is_none() {
        return Ok(());
    }
    let write_cells = estimated_write_cells(method, target, body)?;
    let snapshot = db.tenant_quota_snapshot()?;
    if let Some(limit) = state.options.tenant_max_cells {
        let projected = snapshot.logical_cells.saturating_add(write_cells);
        if write_cells > 0 && projected > limit {
            return Err(RouterError::QuotaExceeded(format!(
                "tenant max cells quota exceeded: projected {projected} > limit {limit}"
            )));
        }
    }
    if let Some(limit) = state.options.tenant_max_memory_bytes {
        let projected = snapshot
            .estimated_memory_bytes
            .saturating_add(body.len() as u64);
        if !body.is_empty() && projected > limit {
            return Err(RouterError::QuotaExceeded(format!(
                "tenant estimated memory quota exceeded: projected {projected} bytes > limit {limit} bytes"
            )));
        }
    }
    Ok(())
}

fn estimated_write_cells(method: &str, target: &str, body: &[u8]) -> Result<u64, RouterError> {
    let (path, _query) = target.split_once('?').unwrap_or((target, ""));
    match (method, path) {
        ("POST", "/put")
        | ("POST", "/v1/cell")
        | ("POST", "/v1/remember")
        | ("POST", "/v1/feedback") => Ok(1),
        ("POST", "/v1/batch") => {
            let request: WriteBatchRequest = serde_json::from_slice(body)
                .map_err(|e| RouterError::BadRequest(format!("invalid batch JSON: {e}")))?;
            Ok(request
                .operations
                .iter()
                .filter(|operation| {
                    matches!(
                        operation,
                        WriteBatchOperationRequest::PutCell { .. }
                            | WriteBatchOperationRequest::PatchCell { .. }
                    )
                })
                .count() as u64)
        }
        ("POST", "/v1/ingest/text") => {
            let text = String::from_utf8_lossy(body);
            Ok(cortex_engine::count_text_chunks(
                "http_post",
                &text,
                cortex_engine::TextChunkPolicy::default(),
            )? as u64)
        }
        ("POST", "/v1/ingest/json") => {
            let json = String::from_utf8_lossy(body);
            Ok(cortex_engine::count_json_ingest_cells(&json)? as u64)
        }
        ("POST", "/v1/ingest/csv") => {
            let csv = String::from_utf8_lossy(body);
            Ok(cortex_engine::count_csv_ingest_cells(&csv)? as u64)
        }
        _ => Ok(0),
    }
}
