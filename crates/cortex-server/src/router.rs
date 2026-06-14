use cortex_aql::{AgentId, AgentView};
use cortex_engine::concurrency::{ReadGuard, WriteGuard};
use cortex_engine::Database;
use std::ops::Deref;

use crate::aql;
use crate::auth::AuthRouteContext;
use crate::authz;
use crate::context;
use crate::hnsw_profile;
use crate::responses::RouterError;
use crate::search;

mod core_routes;
mod ingest_routes;
mod memory_routes;
mod metrics_routes;
mod params;

pub use params::{
    cell_id, json_error, json_response, query_param, query_param_decoded, query_param_opt,
    query_param_opt_decoded,
};

/// Internal trait that lets `route_database_with_auth` run against either a
/// read guard or a write guard. Write routes call `as_write` and fail if the
/// caller only holds a read lock.
pub(crate) trait DatabaseAccess: Deref<Target = Database> {
    fn as_read(&self) -> &Database;
    fn as_write(&mut self) -> Option<&mut Database>;
}

impl DatabaseAccess for &mut Database {
    fn as_read(&self) -> &Database {
        self
    }
    fn as_write(&mut self) -> Option<&mut Database> {
        Some(self)
    }
}

impl<'a> DatabaseAccess for ReadGuard<'a, Database> {
    fn as_read(&self) -> &Database {
        self
    }
    fn as_write(&mut self) -> Option<&mut Database> {
        None
    }
}

impl<'a> DatabaseAccess for WriteGuard<'a, Database> {
    fn as_read(&self) -> &Database {
        self
    }
    fn as_write(&mut self) -> Option<&mut Database> {
        Some(&mut *self)
    }
}

/// Production route entrypoint used by `DatabaseActor`.
pub fn route_database(
    db: &mut Database,
    method: &str,
    target: &str,
    body: &[u8],
) -> Result<String, RouterError> {
    route_database_with_agent(db, method, target, body, None)
}

pub fn route_database_with_agent(
    db: &mut Database,
    method: &str,
    target: &str,
    body: &[u8],
    auth_agent_id: Option<u64>,
) -> Result<String, RouterError> {
    route_database_with_auth(
        db,
        method,
        target,
        body,
        AuthRouteContext::for_agent(auth_agent_id),
    )
}

pub(crate) fn route_database_with_auth<A: DatabaseAccess>(
    mut db: A,
    method: &str,
    target: &str,
    body: &[u8],
    auth_context: AuthRouteContext,
) -> Result<String, RouterError> {
    let (path, query) = target.split_once('?').unwrap_or((target, ""));
    let mut authenticated_view = if is_agent_scoped_route(method, path) {
        authz::load_agent_view(&db, auth_context.agent_id.map(AgentId))?
    } else {
        None
    };
    if let Some(limit) = auth_context.context_budget_tokens {
        if let Some(view) = authenticated_view.as_mut() {
            clamp_view_context_budget(view, limit);
        }
    }
    if let Some(result) = core_routes::try_route(
        &mut db,
        method,
        path,
        query,
        body,
        authenticated_view.as_ref(),
    ) {
        return result;
    }

    match (method, path) {
        ("POST", "/v1/context") => {
            let db = db.as_read();
            return context::handle_context_shared(
                db,
                query,
                body,
                authenticated_view.as_ref(),
                Some(&auth_context),
            );
        }
        ("POST", "/v1/context/trace") => {
            let db = db.as_read();
            return context::handle_context_trace_shared(
                db,
                query,
                body,
                authenticated_view.as_ref(),
                Some(&auth_context),
            );
        }
        ("POST", "/v1/aql") => {
            let db = db.as_read();
            return aql::handle_aql_shared(db, query, body, authenticated_view.as_ref());
        }
        ("POST", "/v1/search") => {
            let db = db.as_read();
            return search::handle_search_shared(db, query, body, authenticated_view.as_ref());
        }
        ("POST", "/v1/search/explain") => {
            let db = db.as_read();
            return search::handle_search_explain_shared(
                db,
                query,
                body,
                authenticated_view.as_ref(),
            );
        }
        ("POST", "/v1/search/ann-evaluate") => {
            let db = db.as_read();
            return search::handle_ann_evaluate_shared(
                db,
                query,
                body,
                authenticated_view.as_ref(),
            );
        }
        ("GET", "/v1/admin/search/hnsw/no-fallback-profile") => {
            let db = db.as_read();
            return hnsw_profile::handle_get(db);
        }
        ("PUT", "/v1/admin/search/hnsw/no-fallback-profile") => {
            let db = db
                .as_write()
                .ok_or_else(|| RouterError::Internal("write route on read lock".to_owned()))?;
            return hnsw_profile::handle_put(db, body);
        }
        ("DELETE", "/v1/admin/search/hnsw/no-fallback-profile") => {
            let db = db
                .as_write()
                .ok_or_else(|| RouterError::Internal("write route on read lock".to_owned()))?;
            return hnsw_profile::handle_delete(db);
        }
        _ => {}
    }

    if let Some(result) = metrics_routes::try_route(&mut db, method, path, query) {
        return result;
    }
    if let Some(result) = memory_routes::try_route(
        &mut db,
        method,
        path,
        query,
        body,
        authenticated_view.as_ref(),
    ) {
        return result;
    }
    if let Some(result) = ingest_routes::try_route(
        &mut db,
        method,
        path,
        query,
        body,
        authenticated_view.as_ref(),
    ) {
        return result;
    }

    Err(RouterError::NotFound("route not found".to_owned()))
}

fn is_agent_scoped_route(method: &str, path: &str) -> bool {
    matches!(
        (method, path),
        ("GET", "/get")
            | ("GET", "/v1/cell")
            | ("POST", "/put")
            | ("POST", "/v1/cell")
            | ("POST", "/v1/batch")
            | ("POST", "/tombstone")
            | ("DELETE", "/v1/cell")
            | ("POST", "/v1/context")
            | ("POST", "/v1/context/trace")
            | ("POST", "/v1/aql")
            | ("POST", "/v1/search")
            | ("POST", "/v1/search/explain")
            | ("POST", "/v1/search/ann-evaluate")
            | ("POST", "/v1/remember")
            | ("POST", "/v1/forget")
            | ("POST", "/v1/verify")
            | ("POST", "/v1/feedback")
            | ("GET", "/v1/feedback/stats")
            | ("GET", "/v1/conflicts")
            | ("POST", "/v1/ingest/text")
            | ("POST", "/v1/ingest/json")
            | ("POST", "/v1/ingest/csv")
    )
}

/// Legacy/test compatibility wrapper that acquires a write lock and delegates to `route_database`.
/// Prefer `route_database` in production paths where the caller already owns exclusive access.
pub fn route_shared(
    db: &std::sync::RwLock<Database>,
    method: &str,
    target: &str,
    body: &[u8],
) -> Result<String, RouterError> {
    route_shared_with_agent(db, method, target, body, None)
}

pub fn route_shared_with_agent(
    db: &std::sync::RwLock<Database>,
    method: &str,
    target: &str,
    body: &[u8],
    auth_agent_id: Option<u64>,
) -> Result<String, RouterError> {
    let mut db = db
        .write()
        .map_err(|e| RouterError::Internal(e.to_string()))?;
    route_database_with_agent(&mut db, method, target, body, auth_agent_id)
}

#[cfg(test)]
pub(crate) fn route_shared_with_auth(
    db: &std::sync::RwLock<Database>,
    method: &str,
    target: &str,
    body: &[u8],
    auth_context: AuthRouteContext,
) -> Result<String, RouterError> {
    let mut db = db
        .write()
        .map_err(|e| RouterError::Internal(e.to_string()))?;
    route_database_with_auth(&mut *db, method, target, body, auth_context)
}

fn clamp_view_context_budget(view: &mut AgentView, limit: u32) {
    view.max_context_budget_tokens = view.max_context_budget_tokens.min(limit);
    view.default_context_budget_tokens = view
        .default_context_budget_tokens
        .min(view.max_context_budget_tokens);
}
