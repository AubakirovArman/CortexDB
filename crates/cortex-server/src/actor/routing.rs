use std::sync::atomic::Ordering;

use crate::auth::AuthRouteContext;
use crate::config::ServerOptions;
use crate::receipt::ReceiptEmissionContext;
use crate::responses::RouterError;
use crate::router::route_database_with_auth;

use super::DatabaseActor;

/// Returns `true` for routes that mutate database state and therefore require a
/// write lock. This classification must stay in sync with the route handlers in
/// `router.rs`.
pub(crate) fn is_write_route(method: &str, target: &str) -> bool {
    let (path, _query) = target.split_once('?').unwrap_or((target, ""));
    matches!(
        (method, path),
        ("POST", "/put")
            | ("POST", "/v1/cell")
            | ("POST", "/v1/batch")
            | ("POST", "/v1/transactions")
            | ("POST", "/v1/handoff")
            | ("POST", "/v1/memory/consolidate/commit")
            | ("POST", "/tombstone")
            | ("DELETE", "/v1/cell")
            | ("POST", "/flush")
            | ("POST", "/v1/flush")
            | ("POST", "/v1/compact")
            | ("POST", "/v1/admin/compact/trigger")
            | ("PUT", "/v1/admin/search/hnsw/no-fallback-profile")
            | ("DELETE", "/v1/admin/search/hnsw/no-fallback-profile")
            | ("POST", "/v1/remember")
            | ("POST", "/v1/feedback")
            | ("POST", "/v1/forget")
            | ("POST", "/v1/ingest/text")
            | ("POST", "/v1/ingest/json")
            | ("POST", "/v1/ingest/csv")
            | ("POST", "/v1/embedding/backfill")
    ) || (method == "DELETE" && path.starts_with("/v1/ingest/jobs/"))
        || (method == "POST"
            && path.starts_with("/v1/ingest/jobs/")
            && (path.ends_with("/cancel") || path.ends_with("/retry")))
}

/// Returns `true` for local operational routes that should use the shorter
/// admin timeout budget instead of the default read budget.
pub(crate) fn is_admin_route(method: &str, target: &str) -> bool {
    let (path, _query) = target.split_once('?').unwrap_or((target, ""));
    matches!(
        (method, path),
        ("GET", "/v1/health")
            | ("GET", "/v1/stats")
            | ("GET", "/v1/metrics")
            | ("GET", "/v1/ann/metrics")
            | ("GET", "/v1/validate")
            | ("GET", "/v1/cluster/status")
            | ("POST", "/v1/backup")
            | ("POST", "/v1/backup/verify")
            | ("POST", "/v1/backup/drill")
            | ("POST", "/v1/backup/offsite/stage")
            | ("POST", "/v1/backup/prune")
    )
}

pub(crate) fn route_timeout_ms(options: &ServerOptions, method: &str, target: &str) -> u64 {
    if is_admin_route(method, target) {
        options.admin_route_timeout_ms()
    } else if is_write_route(method, target) {
        options.write_route_timeout_ms()
    } else {
        options.read_route_timeout_ms()
    }
}

impl DatabaseActor {
    pub fn route(&self, method: &str, target: &str, body: &[u8]) -> Result<String, RouterError> {
        self.route_with_agent(method, target, body, None)
    }

    pub fn route_with_agent(
        &self,
        method: &str,
        target: &str,
        body: &[u8],
        auth_agent_id: Option<u64>,
    ) -> Result<String, RouterError> {
        self.route_with_auth(
            method,
            target,
            body,
            AuthRouteContext::for_agent(auth_agent_id),
            None,
        )
    }

    pub(crate) fn route_with_auth(
        &self,
        method: &str,
        target: &str,
        body: &[u8],
        auth_context: AuthRouteContext,
        receipt_context: Option<&ReceiptEmissionContext>,
    ) -> Result<String, RouterError> {
        self.ensure_open()?;
        let permit = match self.semaphore.try_acquire() {
            Some(permit) => permit,
            None => {
                self.requests_rejected.fetch_add(1, Ordering::Relaxed);
                return Err(RouterError::DatabaseBusy("database actor busy".to_owned()));
            }
        };
        self.requests_sent.fetch_add(1, Ordering::Relaxed);

        let result = if is_write_route(method, target) {
            let guard = self.db.write();
            route_database_with_auth(guard, method, target, body, auth_context, receipt_context)
        } else {
            let guard = self.db.read();
            route_database_with_auth(guard, method, target, body, auth_context, receipt_context)
        };

        drop(permit);
        self.requests_completed.fetch_add(1, Ordering::Relaxed);
        result
    }
}
