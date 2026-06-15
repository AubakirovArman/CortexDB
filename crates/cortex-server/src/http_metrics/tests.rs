use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use crate::config::ServerOptions;
use crate::metrics;
use crate::rate_limit::{PrincipalRateLimits, TenantQueueLimits};
use crate::state::AppState;

use super::record_ann_search_metrics;

fn app_state_for_metrics() -> AppState {
    AppState {
        root: PathBuf::new(),
        dbs: Arc::new(Mutex::new(BTreeMap::new())),
        options: Arc::new(ServerOptions::default()),
        audit_sink: None,
        request_count: Arc::new(AtomicU64::new(0)),
        request_rejected: Arc::new(AtomicU64::new(0)),
        request_timeout: Arc::new(AtomicU64::new(0)),
        request_duration_ms_total: Arc::new(AtomicU64::new(0)),
        request_id_client_provided: Arc::new(AtomicU64::new(0)),
        request_id_generated: Arc::new(AtomicU64::new(0)),
        ann_search_requests: Arc::new(AtomicU64::new(0)),
        ann_fallbacks: Arc::new(AtomicU64::new(0)),
        ann_no_fallback_requests: Arc::new(AtomicU64::new(0)),
        ann_no_fallback_allowed: Arc::new(AtomicU64::new(0)),
        ann_no_fallback_blocked: Arc::new(AtomicU64::new(0)),
        ann_search_latency_ms: metrics::LatencyHistogram::new(),
        actor_queue_wait_latency_ms: metrics::LatencyHistogram::new(),
        validation_failures: Arc::new(AtomicU64::new(0)),
        principal_quota_requests_allowed: Arc::new(AtomicU64::new(0)),
        principal_quota_requests_rejected: Arc::new(AtomicU64::new(0)),
        principal_quota_body_bytes_allowed: Arc::new(AtomicU64::new(0)),
        principal_quota_body_bytes_rejected: Arc::new(AtomicU64::new(0)),
        principal_quota_queue_acquired: Arc::new(AtomicU64::new(0)),
        principal_quota_queue_rejected: Arc::new(AtomicU64::new(0)),
        compactions_triggered: Arc::new(AtomicU64::new(0)),
        compactions_completed: Arc::new(AtomicU64::new(0)),
        compaction_duration_ms_total: Arc::new(AtomicU64::new(0)),
        compaction_cells_compacted: Arc::new(AtomicU64::new(0)),
        compaction_input_bytes: Arc::new(AtomicU64::new(0)),
        compaction_paused: Arc::new(AtomicBool::new(false)),
        rate_limit: None,
        principal_rate_limits: PrincipalRateLimits::default(),
        tenant_queue_limits: TenantQueueLimits::default(),
    }
}

#[test]
fn records_no_fallback_rollout_decision_counters() {
    let state = app_state_for_metrics();
    assert!(record_ann_search_metrics(
        &state,
        r#"{"ann_report":{"fallback_performed":false},"no_fallback_decision":{"allowed":true,"reasons":[]}}"#,
    ));
    assert!(record_ann_search_metrics(
        &state,
        r#"{"ann_report":{"fallback_performed":true},"no_fallback_decision":{"allowed":false,"reasons":["recall_below_minimum"]}}"#,
    ));

    assert_eq!(state.ann_search_requests.load(Ordering::Relaxed), 2);
    assert_eq!(state.ann_fallbacks.load(Ordering::Relaxed), 1);
    assert_eq!(state.ann_no_fallback_requests.load(Ordering::Relaxed), 2);
    assert_eq!(state.ann_no_fallback_allowed.load(Ordering::Relaxed), 1);
    assert_eq!(state.ann_no_fallback_blocked.load(Ordering::Relaxed), 1);
}

#[test]
fn records_ann_search_latency_histogram_buckets() {
    let state = app_state_for_metrics();
    state.ann_search_latency_ms.observe_ms(9);
    state.ann_search_latency_ms.observe_ms(75);
    state.ann_search_latency_ms.observe_ms(1500);

    let buckets = state.ann_search_latency_ms.snapshot();
    assert_eq!(buckets.count, 3);
    assert_eq!(buckets.sum_ms, 1584);
    assert_eq!(buckets.le_10_ms, 1);
    assert_eq!(buckets.le_50_ms, 1);
    assert_eq!(buckets.le_100_ms, 2);
    assert_eq!(buckets.le_500_ms, 2);
    assert_eq!(buckets.le_1000_ms, 2);
    assert_eq!(buckets.gt_1000_ms, 1);
}
