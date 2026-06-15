use std::path::PathBuf;

use cortex_engine::DatabaseOptions;

use crate::auth::{AuthRole, AuthTokenPolicy};

pub const DEFAULT_ACTOR_QUEUE_CAPACITY: usize = 1024;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ServerOptions {
    pub auth_token: Option<String>,
    /// Optional AgentView id bound to the configured bearer token.
    ///
    /// When set, successful HTTP auth loads the persisted `AgentView` with this
    /// id and scope-bound data routes are checked against that view.
    pub auth_agent_id: Option<u64>,
    /// Additional bearer token policies.
    ///
    /// `AuthRole::Admin` can access all authenticated routes. `AuthRole::Data`
    /// can access data routes and health checks, but not admin/metrics routes.
    /// Each policy may bind to a distinct persisted `AgentView`.
    pub auth_tokens: Vec<AuthTokenPolicy>,
    /// Optional file containing one bearer token policy per line.
    ///
    /// The file uses the same `role:token[:agent_id]` entries as
    /// `CORTEXDB_AUTH_TOKENS`, supports `#` comments and blank lines, and is
    /// re-read for every request so operators can rotate tokens without a
    /// server restart. If the configured file is missing or invalid, auth fails
    /// closed.
    pub auth_tokens_file: Option<PathBuf>,
    /// Optional JSON policy store for durable local auth principals.
    ///
    /// The file is re-read for every request and uses
    /// `schema_version = cortexdb.auth_policy.v1`. Disabled principals are
    /// ignored, invalid stores fail closed, and policies may bind principals to
    /// a role plus optional AgentView id.
    pub auth_policy_store_file: Option<PathBuf>,
    /// Capacity of the bounded actor command queue. Default is 1024.
    pub actor_queue_capacity: usize,
    /// Optional per-tenant logical cell limit. Checked before write routes.
    pub tenant_max_cells: Option<u64>,
    /// Optional per-tenant estimated memory limit in bytes. Checked before write routes.
    pub tenant_max_memory_bytes: Option<u64>,
    /// Optional per-tenant in-flight actor command quota.
    pub tenant_queue_quota: Option<u64>,
    /// Optional exact browser origin allowed for cross-origin API requests.
    ///
    /// CORS is disabled by default. Set this only for deployments that expose
    /// CortexDB to a browser origin through a trusted reverse proxy.
    pub cors_allowed_origin: Option<String>,
    /// Optional global request limit per 60-second window.
    ///
    /// Rate limiting is disabled by default. This is a coarse Core Alpha guard,
    /// not a replacement for reverse-proxy quotas or user-aware authorization.
    pub request_rate_limit_per_minute: Option<u64>,
    /// Emit structured audit events through `tracing` for HTTP API responses.
    ///
    /// Disabled by default. Audit events intentionally record route/action
    /// metadata, status, tenant, and duration, but not request bodies or query
    /// strings.
    pub audit_log_enabled: bool,
    /// Optional JSONL file sink for audit events.
    ///
    /// If set, audit events are written to this append-only local file and the
    /// file is synced after each event. The sink stores route metadata only,
    /// never request bodies or query strings.
    pub audit_log_path: Option<PathBuf>,
    /// Enables the deterministic local LLM inference test-double endpoint.
    ///
    /// Disabled by default. This does not enable a production model runtime or
    /// external provider calls.
    pub llm_test_double_enabled: bool,
    /// Enables the embedded developer dashboard.
    ///
    /// Disabled by default so production-safe server defaults expose only typed
    /// API routes. Enable explicitly for local admin consoles or demos.
    pub dashboard_enabled: bool,
    /// Engine database options used for databases opened by this server.
    ///
    /// Defaults to production-safe behavior through `DatabaseOptions::default()`.
    /// Server startup may populate this from `EngineConfig::from_env()`.
    pub engine_database_options: DatabaseOptions,
    /// Enable the background compaction task that periodically tries to merge
    /// selected live segments.
    pub background_compaction_enabled: bool,
    /// Interval in seconds between background compaction passes.
    pub background_compaction_interval_seconds: u64,
}

impl ServerOptions {
    pub fn actor_queue_capacity(&self) -> usize {
        if self.actor_queue_capacity == 0 {
            DEFAULT_ACTOR_QUEUE_CAPACITY
        } else {
            self.actor_queue_capacity
        }
    }

    pub(crate) fn effective_auth_tokens(&self) -> Vec<AuthTokenPolicy> {
        let mut tokens = self.auth_tokens.clone();
        if let Some(token) = &self.auth_token {
            let mut policy = AuthTokenPolicy::new(token.clone(), AuthRole::Admin);
            policy.agent_id = self.auth_agent_id;
            tokens.push(policy);
        }
        tokens
    }
}
