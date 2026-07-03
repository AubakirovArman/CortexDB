mod actor;
mod agent_transaction;
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
mod cluster;
mod config;
mod context;
mod dashboard;
#[cfg(test)]
mod dashboard_tests;
mod database_identity;
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
mod receipt;
mod receipt_signer;
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
pub use config::{
    AuditLogFsyncPolicy, AuditMacKey, ReceiptSigningKey, ServerOptions,
    DEFAULT_ACTOR_QUEUE_CAPACITY, DEFAULT_ADMIN_ROUTE_TIMEOUT_MS,
    DEFAULT_CLUSTER_INGRESS_MAX_IN_FLIGHT_PER_NODE, DEFAULT_READ_ROUTE_TIMEOUT_MS,
    DEFAULT_WRITE_ROUTE_TIMEOUT_MS,
};
pub use lifecycle::{serve, serve_with_options};
pub use receipt_signer::ReceiptExternalSigner;
pub use router::{
    cell_id, json_error, json_response, query_param, query_param_decoded, query_param_opt,
    query_param_opt_decoded, route_database, route_database_with_agent, route_shared,
    route_shared_with_agent,
};
pub use state::{validate_tenant_id, AppState};
#[cfg(test)]
pub use sync_handler::{handle_http, handle_http_with_options};
