use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64};
use std::sync::{Arc, Mutex};

use crate::cluster::ClusterIngressMonitor;
use crate::config::ServerOptions;
use crate::rate_limit::{GlobalRateLimit, PrincipalRateLimits, TenantQueueLimits};
use crate::{actor, audit, metrics};

/// Validates that a tenant ID is safe and conforms to the alphanumeric format,
/// preventing path traversal attacks.
///
/// Allowed characters are ASCII alphanumeric, `_`, and `-`. Length must be between 1 and 64.
/// `:` is intentionally disallowed in tenant IDs because a tenant maps directly
/// to a directory name on disk, whereas `:` is reserved for logical scope
/// namespaces (e.g., `project:investments`). This keeps the filesystem
/// boundary clean and prevents accidental scope/tenant collisions.
pub fn validate_tenant_id(tenant: &str) -> bool {
    if tenant.is_empty() || tenant.len() > 64 {
        return false;
    }
    tenant
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

#[derive(Clone)]
pub struct AppState {
    pub(crate) root: PathBuf,
    pub(crate) dbs: Arc<Mutex<BTreeMap<String, Arc<actor::DatabaseActor>>>>,
    pub(crate) options: Arc<ServerOptions>,
    pub(crate) cluster_ingress_monitor: Option<Arc<ClusterIngressMonitor>>,
    pub(crate) audit_sink: Option<Arc<audit::AuditSink>>,
    pub(crate) request_count: Arc<AtomicU64>,
    pub(crate) request_rejected: Arc<AtomicU64>,
    pub(crate) request_timeout: Arc<AtomicU64>,
    pub(crate) request_duration_ms_total: Arc<AtomicU64>,
    pub(crate) request_id_client_provided: Arc<AtomicU64>,
    pub(crate) request_id_generated: Arc<AtomicU64>,
    pub(crate) ann_search_requests: Arc<AtomicU64>,
    pub(crate) ann_fallbacks: Arc<AtomicU64>,
    pub(crate) ann_no_fallback_requests: Arc<AtomicU64>,
    pub(crate) ann_no_fallback_allowed: Arc<AtomicU64>,
    pub(crate) ann_no_fallback_blocked: Arc<AtomicU64>,
    pub(crate) ann_search_latency_ms: metrics::LatencyHistogram,
    pub(crate) actor_queue_wait_latency_ms: metrics::LatencyHistogram,
    pub(crate) validation_failures: Arc<AtomicU64>,
    pub(crate) principal_quota_requests_allowed: Arc<AtomicU64>,
    pub(crate) principal_quota_requests_rejected: Arc<AtomicU64>,
    pub(crate) principal_quota_body_bytes_allowed: Arc<AtomicU64>,
    pub(crate) principal_quota_body_bytes_rejected: Arc<AtomicU64>,
    pub(crate) principal_quota_queue_acquired: Arc<AtomicU64>,
    pub(crate) principal_quota_queue_rejected: Arc<AtomicU64>,
    pub(crate) compactions_triggered: Arc<AtomicU64>,
    pub(crate) compactions_completed: Arc<AtomicU64>,
    pub(crate) compaction_duration_ms_total: Arc<AtomicU64>,
    pub(crate) compaction_cells_compacted: Arc<AtomicU64>,
    pub(crate) compaction_input_bytes: Arc<AtomicU64>,
    pub(crate) compaction_paused: Arc<AtomicBool>,
    pub(crate) rate_limit: Option<GlobalRateLimit>,
    pub(crate) principal_rate_limits: PrincipalRateLimits,
    pub(crate) tenant_queue_limits: TenantQueueLimits,
}

impl AppState {
    pub(crate) fn db_actor_snapshot(&self) -> Vec<Arc<actor::DatabaseActor>> {
        match self.dbs.lock() {
            Ok(dbs) => dbs.values().cloned().collect(),
            Err(_) => Vec::new(),
        }
    }

    pub fn get_db(&self, tenant: &str) -> std::io::Result<Arc<actor::DatabaseActor>> {
        if !validate_tenant_id(tenant) {
            return Err(std::io::Error::other(format!(
                "invalid tenant id: '{tenant}'"
            )));
        }
        let mut dbs = self
            .dbs
            .lock()
            .map_err(|e| std::io::Error::other(e.to_string()))?;
        if let Some(db) = dbs.get(tenant) {
            return Ok(db.clone());
        }
        let tenant_path = if tenant == "default" {
            self.root.clone()
        } else {
            self.root.join("realms").join(tenant)
        };
        std::fs::create_dir_all(&tenant_path)?;
        let capacity = self.options.actor_queue_capacity();
        let db_shared = Arc::new(actor::DatabaseActor::open_with_capacity_and_options(
            &tenant_path,
            capacity,
            self.options.engine_database_options.clone(),
        )?);
        dbs.insert(tenant.to_owned(), db_shared.clone());
        Ok(db_shared)
    }
}
