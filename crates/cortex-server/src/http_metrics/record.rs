use std::sync::atomic::Ordering;

use crate::state::AppState;

pub(crate) fn record_ann_search_metrics(state: &AppState, body: &str) -> bool {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(body) else {
        return false;
    };
    let Some(report) = value.get("ann_report") else {
        return false;
    };
    if report.is_null() {
        return false;
    }
    state.ann_search_requests.fetch_add(1, Ordering::Relaxed);
    if report
        .get("fallback_performed")
        .and_then(|value| value.as_bool())
        .unwrap_or(false)
    {
        state.ann_fallbacks.fetch_add(1, Ordering::Relaxed);
    }
    let Some(decision) = value.get("no_fallback_decision") else {
        return true;
    };
    if decision.is_null() {
        return true;
    }
    state
        .ann_no_fallback_requests
        .fetch_add(1, Ordering::Relaxed);
    if decision
        .get("allowed")
        .and_then(|value| value.as_bool())
        .unwrap_or(false)
    {
        state
            .ann_no_fallback_allowed
            .fetch_add(1, Ordering::Relaxed);
    } else {
        state
            .ann_no_fallback_blocked
            .fetch_add(1, Ordering::Relaxed);
    }
    true
}

pub(crate) fn record_validation_metrics(state: &AppState, body: &str) {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(body) else {
        return;
    };
    if value.get("ok").and_then(|value| value.as_bool()) == Some(false) {
        state.validation_failures.fetch_add(1, Ordering::Relaxed);
    }
}
