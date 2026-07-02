use std::env;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use serde::Deserialize;

fn main() -> ExitCode {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let args = env::args().collect::<Vec<_>>();
    let [_, root, addr] = args.as_slice() else {
        eprintln!("usage: cortex-server <path> <addr>");
        return ExitCode::FAILURE;
    };
    let actor_queue_capacity = match actor_queue_capacity_from_env() {
        Ok(capacity) => capacity,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::FAILURE;
        }
    };
    let request_rate_limit_per_minute = match request_rate_limit_from_env() {
        Ok(limit) => limit,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::FAILURE;
        }
    };
    let read_route_timeout_ms = match route_timeout_from_env(
        "CORTEXDB_READ_ROUTE_TIMEOUT_MS",
        cortex_server::DEFAULT_READ_ROUTE_TIMEOUT_MS,
    ) {
        Ok(timeout) => timeout,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::FAILURE;
        }
    };
    let write_route_timeout_ms = match route_timeout_from_env(
        "CORTEXDB_WRITE_ROUTE_TIMEOUT_MS",
        cortex_server::DEFAULT_WRITE_ROUTE_TIMEOUT_MS,
    ) {
        Ok(timeout) => timeout,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::FAILURE;
        }
    };
    let admin_route_timeout_ms = match route_timeout_from_env(
        "CORTEXDB_ADMIN_ROUTE_TIMEOUT_MS",
        cortex_server::DEFAULT_ADMIN_ROUTE_TIMEOUT_MS,
    ) {
        Ok(timeout) => timeout,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::FAILURE;
        }
    };
    let tenant_max_cells = match tenant_max_cells_from_env() {
        Ok(limit) => limit,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::FAILURE;
        }
    };
    let tenant_max_memory_bytes = match tenant_max_memory_bytes_from_env() {
        Ok(limit) => limit,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::FAILURE;
        }
    };
    let tenant_queue_quota = match tenant_queue_quota_from_env() {
        Ok(limit) => limit,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::FAILURE;
        }
    };
    let auth_agent_id = match auth_agent_id_from_env() {
        Ok(agent_id) => agent_id,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::FAILURE;
        }
    };
    let auth_token = env::var("CORTEXDB_AUTH_TOKEN").ok();
    if auth_agent_id.is_some() && auth_token.is_none() {
        eprintln!("CORTEXDB_AUTH_AGENT_ID requires CORTEXDB_AUTH_TOKEN");
        return ExitCode::FAILURE;
    }
    let auth_tokens = match auth_tokens_from_env() {
        Ok(tokens) => tokens,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::FAILURE;
        }
    };
    let auth_tokens_file = match auth_tokens_file_from_env() {
        Ok(path) => path,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::FAILURE;
        }
    };
    let auth_policy_store_file = match auth_policy_store_file_from_env() {
        Ok(path) => path,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::FAILURE;
        }
    };
    let audit_log_path = match audit_log_path_from_env() {
        Ok(path) => path,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::FAILURE;
        }
    };
    let audit_log_rotate_bytes = match audit_log_rotate_bytes_from_env() {
        Ok(limit) => limit,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::FAILURE;
        }
    };
    let audit_log_fsync_policy = match audit_log_fsync_policy_from_env() {
        Ok(policy) => policy,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::FAILURE;
        }
    };
    let audit_log_mac_key = match audit_log_mac_key_from_env() {
        Ok(key) => key,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::FAILURE;
        }
    };
    if audit_log_path.is_some() && audit_log_mac_key.is_none() {
        eprintln!("CORTEXDB_AUDIT_MAC_KEY_HEX is required when CORTEXDB_AUDIT_LOG_FILE is set");
        return ExitCode::FAILURE;
    }
    let receipt_signing_key = match receipt_signing_key_from_env() {
        Ok(key) => key,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::FAILURE;
        }
    };
    let receipt_external_signer = match receipt_external_signer_from_env() {
        Ok(signer) => signer,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::FAILURE;
        }
    };
    if receipt_signing_key.is_some() && receipt_external_signer.is_some() {
        eprintln!(
            "set only one of local receipt signing key config or CORTEXDB_RECEIPT_EXTERNAL_SIGNER_COMMAND"
        );
        return ExitCode::FAILURE;
    }
    let engine_config = match cortex_engine::EngineConfig::from_env() {
        Ok(config) => config,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::FAILURE;
        }
    };
    let cluster_config = match cluster_config_from_env() {
        Ok(config) => config,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::FAILURE;
        }
    };
    let cluster_ingress_leader = match cluster_ingress_leader_from_env() {
        Ok(value) => value,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::FAILURE;
        }
    };
    let cluster_ingress_max_in_flight_per_node = match cluster_ingress_max_in_flight_from_env() {
        Ok(value) => value,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::FAILURE;
        }
    };
    let options = cortex_server::ServerOptions {
        auth_token,
        auth_agent_id,
        auth_tokens,
        auth_tokens_file,
        auth_policy_store_file,
        actor_queue_capacity,
        tenant_max_cells,
        tenant_max_memory_bytes,
        tenant_queue_quota,
        cors_allowed_origin: env::var("CORTEXDB_CORS_ALLOW_ORIGIN").ok(),
        request_rate_limit_per_minute,
        read_route_timeout_ms,
        write_route_timeout_ms,
        admin_route_timeout_ms,
        audit_log_enabled: audit_log_enabled_from_env() || audit_log_path.is_some(),
        audit_log_path,
        audit_log_rotate_bytes,
        audit_log_fsync_policy,
        audit_log_mac_key,
        receipt_signing_key,
        receipt_external_signer,
        db_instance_id: None,
        cluster_config,
        cluster_ingress_leader,
        cluster_ingress_max_in_flight_per_node,
        llm_test_double_enabled: llm_test_double_enabled_from_env(),
        dashboard_enabled: engine_config.database_options.feature_flags.dashboard,
        engine_database_options: engine_config.database_options,
        background_compaction_enabled: false,
        background_compaction_interval_seconds: 60,
    };
    match cortex_server::serve_with_options(&PathBuf::from(root), addr, options) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

fn cluster_config_from_env() -> Result<Option<cortex_engine::ClusterConfig>, String> {
    let path = match env::var("CORTEXDB_CLUSTER_CONFIG_FILE") {
        Ok(raw) => parse_non_empty_path(&raw, "CORTEXDB_CLUSTER_CONFIG_FILE")?,
        Err(env::VarError::NotPresent) => return Ok(None),
        Err(error) => return Err(format!("invalid CORTEXDB_CLUSTER_CONFIG_FILE: {error}")),
    };
    cortex_engine::ClusterConfig::load(&path)
        .map(Some)
        .map_err(|error| {
            format!(
                "invalid CORTEXDB_CLUSTER_CONFIG_FILE {}: {error}",
                path.display()
            )
        })
}

fn cluster_ingress_leader_from_env() -> Result<Option<cortex_engine::NodeId>, String> {
    match env::var("CORTEXDB_CLUSTER_INGRESS_LEADER_ID") {
        Ok(raw) => parse_cluster_ingress_leader(&raw).map(Some),
        Err(env::VarError::NotPresent) => Ok(None),
        Err(error) => Err(format!(
            "invalid CORTEXDB_CLUSTER_INGRESS_LEADER_ID: {error}"
        )),
    }
}

fn parse_cluster_ingress_leader(raw: &str) -> Result<cortex_engine::NodeId, String> {
    parse_positive_u64(raw, "CORTEXDB_CLUSTER_INGRESS_LEADER_ID").map(cortex_engine::NodeId)
}

fn cluster_ingress_max_in_flight_from_env() -> Result<usize, String> {
    match env::var("CORTEXDB_CLUSTER_INGRESS_MAX_IN_FLIGHT_PER_NODE") {
        Ok(raw) => parse_cluster_ingress_max_in_flight(&raw),
        Err(env::VarError::NotPresent) => {
            Ok(cortex_server::DEFAULT_CLUSTER_INGRESS_MAX_IN_FLIGHT_PER_NODE)
        }
        Err(error) => Err(format!(
            "invalid CORTEXDB_CLUSTER_INGRESS_MAX_IN_FLIGHT_PER_NODE: {error}"
        )),
    }
}

fn parse_cluster_ingress_max_in_flight(raw: &str) -> Result<usize, String> {
    parse_positive_usize(raw, "CORTEXDB_CLUSTER_INGRESS_MAX_IN_FLIGHT_PER_NODE")
}

fn auth_tokens_from_env() -> Result<Vec<cortex_server::AuthTokenPolicy>, String> {
    match env::var("CORTEXDB_AUTH_TOKENS") {
        Ok(raw) => parse_auth_tokens(&raw),
        Err(env::VarError::NotPresent) => Ok(Vec::new()),
        Err(error) => Err(format!("invalid CORTEXDB_AUTH_TOKENS: {error}")),
    }
}

fn parse_auth_tokens(raw: &str) -> Result<Vec<cortex_server::AuthTokenPolicy>, String> {
    cortex_server::parse_auth_tokens(raw)
        .map_err(|error| format!("invalid CORTEXDB_AUTH_TOKENS: {error}"))
}

fn auth_tokens_file_from_env() -> Result<Option<PathBuf>, String> {
    match env::var("CORTEXDB_AUTH_TOKENS_FILE") {
        Ok(raw) => parse_auth_tokens_file_path(&raw).map(Some),
        Err(env::VarError::NotPresent) => Ok(None),
        Err(error) => Err(format!("invalid CORTEXDB_AUTH_TOKENS_FILE: {error}")),
    }
}

fn auth_policy_store_file_from_env() -> Result<Option<PathBuf>, String> {
    match env::var("CORTEXDB_AUTH_POLICY_STORE_FILE") {
        Ok(raw) => parse_auth_policy_store_file_path(&raw).map(Some),
        Err(env::VarError::NotPresent) => Ok(None),
        Err(error) => Err(format!("invalid CORTEXDB_AUTH_POLICY_STORE_FILE: {error}")),
    }
}

fn parse_auth_tokens_file_path(raw: &str) -> Result<PathBuf, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        Err("CORTEXDB_AUTH_TOKENS_FILE must not be empty".to_owned())
    } else {
        Ok(PathBuf::from(trimmed))
    }
}

fn parse_auth_policy_store_file_path(raw: &str) -> Result<PathBuf, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        Err("CORTEXDB_AUTH_POLICY_STORE_FILE must not be empty".to_owned())
    } else {
        Ok(PathBuf::from(trimmed))
    }
}

fn audit_log_path_from_env() -> Result<Option<PathBuf>, String> {
    match env::var("CORTEXDB_AUDIT_LOG_FILE") {
        Ok(raw) => parse_audit_log_path(&raw).map(Some),
        Err(env::VarError::NotPresent) => Ok(None),
        Err(error) => Err(format!("invalid CORTEXDB_AUDIT_LOG_FILE: {error}")),
    }
}

fn parse_audit_log_path(raw: &str) -> Result<PathBuf, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        Err("CORTEXDB_AUDIT_LOG_FILE must not be empty".to_owned())
    } else {
        Ok(PathBuf::from(trimmed))
    }
}

fn audit_log_rotate_bytes_from_env() -> Result<Option<u64>, String> {
    optional_positive_u64_from_env("CORTEXDB_AUDIT_LOG_ROTATE_BYTES")
}

fn audit_log_fsync_policy_from_env() -> Result<cortex_server::AuditLogFsyncPolicy, String> {
    match env::var("CORTEXDB_AUDIT_LOG_FSYNC") {
        Ok(raw) => parse_audit_log_fsync_policy(&raw),
        Err(env::VarError::NotPresent) => Ok(cortex_server::AuditLogFsyncPolicy::Always),
        Err(error) => Err(format!("invalid CORTEXDB_AUDIT_LOG_FSYNC: {error}")),
    }
}

fn parse_audit_log_fsync_policy(raw: &str) -> Result<cortex_server::AuditLogFsyncPolicy, String> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "always" => Ok(cortex_server::AuditLogFsyncPolicy::Always),
        "flush" | "flush_only" | "flush-only" => Ok(cortex_server::AuditLogFsyncPolicy::FlushOnly),
        _ => Err("CORTEXDB_AUDIT_LOG_FSYNC must be always or flush".to_owned()),
    }
}

fn audit_log_mac_key_from_env() -> Result<Option<cortex_server::AuditMacKey>, String> {
    let raw_key = match env::var("CORTEXDB_AUDIT_MAC_KEY_HEX") {
        Ok(raw) => raw,
        Err(env::VarError::NotPresent) => return Ok(None),
        Err(error) => return Err(format!("invalid CORTEXDB_AUDIT_MAC_KEY_HEX: {error}")),
    };
    let key_id = match env::var("CORTEXDB_AUDIT_MAC_KEY_ID") {
        Ok(raw) => raw,
        Err(env::VarError::NotPresent) => "local-audit-key".to_owned(),
        Err(error) => return Err(format!("invalid CORTEXDB_AUDIT_MAC_KEY_ID: {error}")),
    };
    parse_audit_log_mac_key(&key_id, &raw_key).map(Some)
}

fn parse_audit_log_mac_key(
    key_id: &str,
    raw_key: &str,
) -> Result<cortex_server::AuditMacKey, String> {
    cortex_server::AuditMacKey::from_hex(key_id.trim(), raw_key)
        .map_err(|error| format!("invalid CORTEXDB_AUDIT_MAC_KEY_HEX: {error}"))
}

fn receipt_signing_key_from_env() -> Result<Option<cortex_server::ReceiptSigningKey>, String> {
    let key_file = match env::var("CORTEXDB_RECEIPT_SIGNING_KEY_FILE") {
        Ok(raw) => Some(parse_non_empty_path(
            &raw,
            "CORTEXDB_RECEIPT_SIGNING_KEY_FILE",
        )?),
        Err(env::VarError::NotPresent) => None,
        Err(error) => {
            return Err(format!(
                "invalid CORTEXDB_RECEIPT_SIGNING_KEY_FILE: {error}"
            ))
        }
    };
    let raw_seed = match env::var("CORTEXDB_RECEIPT_SIGNING_KEY_HEX") {
        Ok(raw) => Some(raw),
        Err(env::VarError::NotPresent) => None,
        Err(error) => return Err(format!("invalid CORTEXDB_RECEIPT_SIGNING_KEY_HEX: {error}")),
    };
    if key_file.is_some() && raw_seed.is_some() {
        return Err(
            "set only one of CORTEXDB_RECEIPT_SIGNING_KEY_FILE or CORTEXDB_RECEIPT_SIGNING_KEY_HEX"
                .to_owned(),
        );
    }
    if let Some(path) = key_file {
        return parse_receipt_signing_key_file(&path).map(Some);
    }
    let Some(raw_seed) = raw_seed else {
        return Ok(None);
    };
    let key_id = match env::var("CORTEXDB_RECEIPT_SIGNING_KEY_ID") {
        Ok(raw) => raw,
        Err(env::VarError::NotPresent) => "local-receipt-key".to_owned(),
        Err(error) => return Err(format!("invalid CORTEXDB_RECEIPT_SIGNING_KEY_ID: {error}")),
    };
    parse_receipt_signing_key(&key_id, &raw_seed).map(Some)
}

fn parse_receipt_signing_key(
    key_id: &str,
    raw_seed: &str,
) -> Result<cortex_server::ReceiptSigningKey, String> {
    cortex_server::ReceiptSigningKey::from_seed_hex(key_id.trim(), raw_seed)
        .map_err(|error| format!("invalid CORTEXDB_RECEIPT_SIGNING_KEY_HEX: {error}"))
}

fn parse_receipt_signing_key_file(path: &Path) -> Result<cortex_server::ReceiptSigningKey, String> {
    let raw = std::fs::read_to_string(path).map_err(|error| {
        format!(
            "failed to read CORTEXDB_RECEIPT_SIGNING_KEY_FILE {}: {error}",
            path.display()
        )
    })?;
    parse_receipt_signing_key_file_json(&raw)
}

#[derive(Deserialize)]
struct ReceiptSigningKeyFile {
    schema_version: String,
    key_id: String,
    signing_seed_hex: String,
    public_key_hex: String,
}

fn parse_receipt_signing_key_file_json(
    raw: &str,
) -> Result<cortex_server::ReceiptSigningKey, String> {
    let file: ReceiptSigningKeyFile = serde_json::from_str(raw)
        .map_err(|error| format!("invalid CORTEXDB_RECEIPT_SIGNING_KEY_FILE JSON: {error}"))?;
    if file.schema_version != "cortexdb.receipt_signing_key.v1" {
        return Err("CORTEXDB_RECEIPT_SIGNING_KEY_FILE schema_version must be cortexdb.receipt_signing_key.v1".to_owned());
    }
    let key = cortex_server::ReceiptSigningKey::from_seed_hex(
        file.key_id.trim(),
        file.signing_seed_hex.trim(),
    )
    .map_err(|error| format!("invalid CORTEXDB_RECEIPT_SIGNING_KEY_FILE: {error}"))?;
    if key.public_key_hex() != file.public_key_hex.trim() {
        return Err(
            "CORTEXDB_RECEIPT_SIGNING_KEY_FILE public_key_hex does not match signing seed"
                .to_owned(),
        );
    }
    Ok(key)
}

fn receipt_external_signer_from_env() -> Result<Option<cortex_server::ReceiptExternalSigner>, String>
{
    let command = match env::var("CORTEXDB_RECEIPT_EXTERNAL_SIGNER_COMMAND") {
        Ok(raw) => Some(parse_non_empty_path(
            &raw,
            "CORTEXDB_RECEIPT_EXTERNAL_SIGNER_COMMAND",
        )?),
        Err(env::VarError::NotPresent) => None,
        Err(error) => {
            return Err(format!(
                "invalid CORTEXDB_RECEIPT_EXTERNAL_SIGNER_COMMAND: {error}"
            ))
        }
    };
    let key_id = env::var("CORTEXDB_RECEIPT_EXTERNAL_SIGNER_KEY_ID").ok();
    let public_key_hex = env::var("CORTEXDB_RECEIPT_EXTERNAL_SIGNER_PUBLIC_KEY_HEX").ok();
    let signer_ref = env::var("CORTEXDB_RECEIPT_EXTERNAL_SIGNER_REF").ok();

    let Some(command) = command else {
        if key_id.is_some() || public_key_hex.is_some() || signer_ref.is_some() {
            return Err("CORTEXDB_RECEIPT_EXTERNAL_SIGNER_COMMAND is required when receipt external signer metadata is set".to_owned());
        }
        return Ok(None);
    };
    let key_id =
        key_id.ok_or_else(|| "CORTEXDB_RECEIPT_EXTERNAL_SIGNER_KEY_ID is required".to_owned())?;
    let public_key_hex = public_key_hex
        .ok_or_else(|| "CORTEXDB_RECEIPT_EXTERNAL_SIGNER_PUBLIC_KEY_HEX is required".to_owned())?;
    parse_receipt_external_signer(&command, &key_id, &public_key_hex, signer_ref).map(Some)
}

fn parse_receipt_external_signer(
    command: &Path,
    key_id: &str,
    public_key_hex: &str,
    signer_ref: Option<String>,
) -> Result<cortex_server::ReceiptExternalSigner, String> {
    cortex_server::ReceiptExternalSigner::new(
        key_id.trim(),
        public_key_hex.trim(),
        command.to_path_buf(),
        signer_ref
            .map(|raw| raw.trim().to_owned())
            .filter(|raw| !raw.is_empty()),
    )
    .map_err(|error| format!("invalid CORTEXDB_RECEIPT_EXTERNAL_SIGNER_PUBLIC_KEY_HEX: {error}"))
}

fn parse_non_empty_path(raw: &str, name: &str) -> Result<PathBuf, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        Err(format!("{name} must not be empty"))
    } else {
        Ok(PathBuf::from(trimmed))
    }
}

fn auth_agent_id_from_env() -> Result<Option<u64>, String> {
    match env::var("CORTEXDB_AUTH_AGENT_ID") {
        Ok(raw) => parse_auth_agent_id(&raw).map(Some),
        Err(env::VarError::NotPresent) => Ok(None),
        Err(error) => Err(format!("invalid CORTEXDB_AUTH_AGENT_ID: {error}")),
    }
}

fn parse_auth_agent_id(raw: &str) -> Result<u64, String> {
    parse_positive_u64(raw, "CORTEXDB_AUTH_AGENT_ID")
}

fn parse_positive_u64(raw: &str, name: &str) -> Result<u64, String> {
    let id = raw
        .parse::<u64>()
        .map_err(|_| format!("{name} must be a positive integer"))?;
    if id == 0 {
        return Err(format!("{name} must be greater than zero"));
    }
    Ok(id)
}

fn parse_positive_usize(raw: &str, name: &str) -> Result<usize, String> {
    let value = raw
        .parse::<usize>()
        .map_err(|_| format!("{name} must be a positive integer"))?;
    if value == 0 {
        return Err(format!("{name} must be greater than zero"));
    }
    Ok(value)
}

fn audit_log_enabled_from_env() -> bool {
    match env::var("CORTEXDB_AUDIT_LOG") {
        Ok(raw) => parse_bool_flag(&raw),
        Err(_) => false,
    }
}

fn llm_test_double_enabled_from_env() -> bool {
    match env::var("CORTEXDB_LLM_TEST_DOUBLE") {
        Ok(raw) => parse_bool_flag(&raw),
        Err(_) => false,
    }
}

fn parse_bool_flag(raw: &str) -> bool {
    matches!(
        raw.trim().to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "on"
    )
}

fn request_rate_limit_from_env() -> Result<Option<u64>, String> {
    match env::var("CORTEXDB_RATE_LIMIT_PER_MINUTE") {
        Ok(raw) => parse_request_rate_limit(&raw).map(Some),
        Err(env::VarError::NotPresent) => Ok(None),
        Err(error) => Err(format!("invalid CORTEXDB_RATE_LIMIT_PER_MINUTE: {error}")),
    }
}

fn parse_request_rate_limit(raw: &str) -> Result<u64, String> {
    parse_positive_u64(raw, "CORTEXDB_RATE_LIMIT_PER_MINUTE")
}

fn route_timeout_from_env(var: &str, default_ms: u64) -> Result<u64, String> {
    match env::var(var) {
        Ok(raw) => parse_positive_u64(&raw, var),
        Err(env::VarError::NotPresent) => Ok(default_ms),
        Err(error) => Err(format!("invalid {var}: {error}")),
    }
}

fn tenant_max_cells_from_env() -> Result<Option<u64>, String> {
    optional_positive_u64_from_env("CORTEXDB_TENANT_MAX_CELLS")
}

fn tenant_max_memory_bytes_from_env() -> Result<Option<u64>, String> {
    optional_positive_u64_from_env("CORTEXDB_TENANT_MAX_MEMORY_BYTES")
}

fn tenant_queue_quota_from_env() -> Result<Option<u64>, String> {
    optional_positive_u64_from_env("CORTEXDB_TENANT_QUEUE_QUOTA")
}

fn optional_positive_u64_from_env(var: &str) -> Result<Option<u64>, String> {
    match env::var(var) {
        Ok(raw) => parse_positive_u64(&raw, var).map(Some),
        Err(env::VarError::NotPresent) => Ok(None),
        Err(error) => Err(format!("invalid {var}: {error}")),
    }
}

fn actor_queue_capacity_from_env() -> Result<usize, String> {
    match env::var("CORTEXDB_ACTOR_QUEUE_CAPACITY") {
        Ok(raw) => parse_actor_queue_capacity(&raw),
        Err(env::VarError::NotPresent) => Ok(cortex_server::DEFAULT_ACTOR_QUEUE_CAPACITY),
        Err(error) => Err(format!("invalid CORTEXDB_ACTOR_QUEUE_CAPACITY: {error}")),
    }
}

fn parse_actor_queue_capacity(raw: &str) -> Result<usize, String> {
    parse_positive_usize(raw, "CORTEXDB_ACTOR_QUEUE_CAPACITY")
}

#[cfg(test)]
#[path = "main/tests.rs"]
mod tests;
