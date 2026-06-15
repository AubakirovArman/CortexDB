use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

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
    let engine_config = match cortex_engine::EngineConfig::from_env() {
        Ok(config) => config,
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
    let capacity = raw
        .parse::<usize>()
        .map_err(|_| "CORTEXDB_ACTOR_QUEUE_CAPACITY must be a positive integer".to_owned())?;
    if capacity == 0 {
        return Err("CORTEXDB_ACTOR_QUEUE_CAPACITY must be greater than zero".to_owned());
    }
    Ok(capacity)
}

#[cfg(test)]
#[path = "main/tests.rs"]
mod tests;
