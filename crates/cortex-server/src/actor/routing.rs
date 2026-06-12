use std::sync::atomic::Ordering;

use crate::auth::AuthRouteContext;
use crate::responses::RouterError;
use crate::router::route_database_with_auth;

use super::DatabaseActor;

/// Returns `true` for routes that mutate database state and therefore require a
/// write lock. This classification must stay in sync with the route handlers in
/// `router.rs`.
pub(super) fn is_write_route(method: &str, target: &str) -> bool {
    let (path, _query) = target.split_once('?').unwrap_or((target, ""));
    matches!(
        (method, path),
        ("POST", "/put")
            | ("POST", "/v1/cell")
            | ("POST", "/tombstone")
            | ("DELETE", "/v1/cell")
            | ("POST", "/flush")
            | ("POST", "/v1/flush")
            | ("POST", "/v1/compact")
            | ("POST", "/v1/admin/compact/trigger")
            | ("PUT", "/v1/admin/search/hnsw/no-fallback-profile")
            | ("DELETE", "/v1/admin/search/hnsw/no-fallback-profile")
            | ("POST", "/v1/remember")
            | ("POST", "/v1/forget")
            | ("POST", "/v1/ingest/text")
            | ("POST", "/v1/ingest/json")
            | ("POST", "/v1/ingest/csv")
    ) || (method == "DELETE" && path.starts_with("/v1/ingest/jobs/"))
        || (method == "POST"
            && path.starts_with("/v1/ingest/jobs/")
            && (path.ends_with("/cancel") || path.ends_with("/retry")))
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
        )
    }

    pub(crate) fn route_with_auth(
        &self,
        method: &str,
        target: &str,
        body: &[u8],
        auth_context: AuthRouteContext,
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
            route_database_with_auth(guard, method, target, body, auth_context)
        } else {
            let guard = self.db.read();
            route_database_with_auth(guard, method, target, body, auth_context)
        };

        drop(permit);
        self.requests_completed.fetch_add(1, Ordering::Relaxed);
        result
    }
}
