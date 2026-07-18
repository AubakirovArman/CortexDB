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
    crate::route_registry::route_spec(method, target).mutating
}

/// Returns `true` for local operational routes that should use the shorter
/// admin timeout budget instead of the default read budget.
pub(crate) fn is_admin_route(method: &str, target: &str) -> bool {
    crate::route_registry::route_spec(method, target).timeout
        == crate::route_registry::RouteTimeoutClass::Admin
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

/// Returns an actor execution deadline only when abandoning the response
/// cannot leave a mutation running with an unknown commit outcome.
pub(crate) fn actor_route_timeout_ms(
    options: &ServerOptions,
    method: &str,
    target: &str,
) -> Option<u64> {
    (!is_write_route(method, target)).then(|| route_timeout_ms(options, method, target))
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
