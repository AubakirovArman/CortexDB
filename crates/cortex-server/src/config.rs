use std::fmt;
use std::path::PathBuf;
use std::sync::Arc;

use cortex_crypto::{KeyId, MacKey, SigningSeed};
use cortex_engine::{ClusterConfig, DatabaseOptions, NodeId};

use crate::auth::{AuthRole, AuthTokenPolicy};
use crate::receipt_signer::ReceiptExternalSigner;

pub const DEFAULT_ACTOR_QUEUE_CAPACITY: usize = 1024;
pub const DEFAULT_READ_ROUTE_TIMEOUT_MS: u64 = 30_000;
pub const DEFAULT_WRITE_ROUTE_TIMEOUT_MS: u64 = 30_000;
pub const DEFAULT_ADMIN_ROUTE_TIMEOUT_MS: u64 = 10_000;
pub const DEFAULT_CLUSTER_INGRESS_MAX_IN_FLIGHT_PER_NODE: usize = 64;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AuditLogFsyncPolicy {
    #[default]
    Always,
    FlushOnly,
}

#[derive(Clone, PartialEq, Eq)]
pub struct AuditMacKey {
    key_id: KeyId,
    secret: Arc<MacKey>,
}

impl AuditMacKey {
    pub fn from_hex(key_id: &str, raw_hex: &str) -> Result<Self, String> {
        let key_id = KeyId::new(key_id.to_owned()).map_err(|error| error.to_string())?;
        let bytes = decode_hex_32(raw_hex, "audit MAC key")?;
        let secret =
            MacKey::from_slice("audit MAC key", &bytes).map_err(|error| error.to_string())?;
        Ok(Self {
            key_id,
            secret: Arc::new(secret),
        })
    }

    pub fn key_id(&self) -> &str {
        self.key_id.as_str()
    }

    pub(crate) fn mac_key(&self) -> &MacKey {
        &self.secret
    }
}

impl fmt::Debug for AuditMacKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AuditMacKey")
            .field("key_id", &self.key_id)
            .field("secret", &"redacted")
            .finish()
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct ReceiptSigningKey {
    key_id: KeyId,
    seed: Arc<SigningSeed>,
}

impl ReceiptSigningKey {
    pub fn from_seed_hex(key_id: &str, raw_hex: &str) -> Result<Self, String> {
        let key_id = KeyId::new(key_id.to_owned()).map_err(|error| error.to_string())?;
        let bytes = decode_hex_32(raw_hex, "receipt signing seed")?;
        let seed = SigningSeed::from_slice("receipt signing seed", &bytes)
            .map_err(|error| error.to_string())?;
        Ok(Self {
            key_id,
            seed: Arc::new(seed),
        })
    }

    pub fn key_id(&self) -> &str {
        self.key_id.as_str()
    }

    pub fn public_key_hex(&self) -> String {
        cortex_crypto::hex_lower(&cortex_crypto::ed25519_public_key(&self.seed))
    }

    pub(crate) fn to_crypto_key(&self) -> cortex_crypto::ReceiptSigningKey {
        let seed = SigningSeed::from_slice("receipt signing seed", self.seed.as_bytes())
            .expect("stored receipt signing seed is always 32 bytes");
        cortex_crypto::ReceiptSigningKey::from_seed(self.key_id.clone(), seed)
    }
}

impl fmt::Debug for ReceiptSigningKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ReceiptSigningKey")
            .field("key_id", &self.key_id)
            .field("seed", &"redacted")
            .finish()
    }
}

fn decode_hex_32(raw_hex: &str, name: &str) -> Result<[u8; 32], String> {
    let raw_hex = raw_hex.trim();
    if raw_hex.len() != 64 {
        return Err(format!(
            "{name} must be 64 lowercase or uppercase hex characters"
        ));
    }
    let mut bytes = [0_u8; 32];
    for (index, chunk) in raw_hex.as_bytes().chunks_exact(2).enumerate() {
        let high = hex_nibble(chunk[0]).ok_or_else(|| format!("{name} contains non-hex data"))?;
        let low = hex_nibble(chunk[1]).ok_or_else(|| format!("{name} contains non-hex data"))?;
        bytes[index] = (high << 4) | low;
    }
    Ok(bytes)
}

fn hex_nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

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
    /// Timeout budget for read-only database routes.
    pub read_route_timeout_ms: u64,
    /// Timeout budget for write/mutation database routes.
    pub write_route_timeout_ms: u64,
    /// Timeout budget for local admin/metrics style database routes.
    pub admin_route_timeout_ms: u64,
    /// Emit structured audit events through `tracing` for HTTP API responses.
    ///
    /// Disabled by default. Audit events intentionally record route/action
    /// metadata, status, tenant, and duration, but not request bodies or query
    /// strings.
    pub audit_log_enabled: bool,
    /// Optional JSONL file sink for audit events.
    ///
    /// If set, audit events are written to this append-only local file. The
    /// sink stores route metadata only, never request bodies or query strings.
    pub audit_log_path: Option<PathBuf>,
    /// Optional active audit JSONL size limit in bytes before rotation.
    pub audit_log_rotate_bytes: Option<u64>,
    /// File durability policy for the audit JSONL sink.
    pub audit_log_fsync_policy: AuditLogFsyncPolicy,
    /// Required 32-byte MAC key for the persisted audit JSONL chain.
    ///
    /// File audit uses this key to emit `cortexdb.audit.v2` records with
    /// HMAC-SHA-256. The key is intentionally not logged or serialized.
    pub audit_log_mac_key: Option<AuditMacKey>,
    /// Optional local Ed25519 node key for accountability receipt signing.
    pub receipt_signing_key: Option<ReceiptSigningKey>,
    /// Optional external command signer for accountability receipt signing.
    pub receipt_external_signer: Option<ReceiptExternalSigner>,
    /// Durable database-instance identity used in accountability receipt headers.
    ///
    /// Server startup loads or creates this from the database root whenever
    /// receipt signing is configured.
    pub db_instance_id: Option<String>,
    /// Optional persisted cluster topology known to this server process.
    ///
    /// Multi-node topologies are exposed through `/v1/cluster/status`. Accountable
    /// context ingress uses the first configured node as a fixed primary forwarding
    /// target and fails closed when that target is unavailable.
    pub cluster_config: Option<ClusterConfig>,
    /// Optional operator-provided context-ingress leader override.
    ///
    /// When set, accountable context ingress forwards to this configured node
    /// instead of falling back to the first node in `cluster_config`. This is
    /// an explicit operator hint, not automatic Raft leader discovery.
    pub cluster_ingress_leader: Option<NodeId>,
    /// Maximum cached-monitor forwarded context-ingress routes in flight per
    /// selected Raft leader. Defaults to 64.
    pub cluster_ingress_max_in_flight_per_node: usize,
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

    pub fn read_route_timeout_ms(&self) -> u64 {
        if self.read_route_timeout_ms == 0 {
            DEFAULT_READ_ROUTE_TIMEOUT_MS
        } else {
            self.read_route_timeout_ms
        }
    }

    pub fn write_route_timeout_ms(&self) -> u64 {
        if self.write_route_timeout_ms == 0 {
            DEFAULT_WRITE_ROUTE_TIMEOUT_MS
        } else {
            self.write_route_timeout_ms
        }
    }

    pub fn admin_route_timeout_ms(&self) -> u64 {
        if self.admin_route_timeout_ms == 0 {
            DEFAULT_ADMIN_ROUTE_TIMEOUT_MS
        } else {
            self.admin_route_timeout_ms
        }
    }

    pub(crate) fn cluster_ingress_max_in_flight_per_node(&self) -> usize {
        if self.cluster_ingress_max_in_flight_per_node == 0 {
            DEFAULT_CLUSTER_INGRESS_MAX_IN_FLIGHT_PER_NODE
        } else {
            self.cluster_ingress_max_in_flight_per_node
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
