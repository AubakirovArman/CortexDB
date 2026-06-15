mod actor;
mod aql;
mod audit;
mod audit_chain;
#[cfg(test)]
mod audit_tests;
mod auth;
mod auth_agent_admin;
mod auth_capability;
mod auth_policy_cells;
mod auth_policy_io;
mod auth_policy_store;
mod auth_scope_admin;
mod authz;
mod config;
mod context;
mod dashboard;
#[cfg(test)]
mod dashboard_tests;
mod embedding;
pub mod external_identity;
mod handler;
mod hnsw_profile;
mod http_metrics;
mod lifecycle;
mod llm;
mod memory;
mod metrics;
mod quota;
mod rate_limit;
mod request_audit;
mod request_id;
pub mod responses;
mod router;
mod search;
#[cfg(test)]
mod search_tests;
mod state;
#[cfg(test)]
mod sync_handler;
#[cfg(test)]
mod tests;

pub use auth::{parse_auth_tokens, AuthRole, AuthTokenPolicy};
pub use config::{AuditLogFsyncPolicy, ServerOptions, DEFAULT_ACTOR_QUEUE_CAPACITY};
pub use lifecycle::{serve, serve_with_options};
pub use router::{
    cell_id, json_error, json_response, query_param, query_param_decoded, query_param_opt,
    query_param_opt_decoded, route_database, route_database_with_agent, route_shared,
    route_shared_with_agent,
};
pub use state::{validate_tenant_id, AppState};
#[cfg(test)]
pub use sync_handler::{handle_http, handle_http_with_options};
