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
        cors_allowed_origin: env::var("CORTEXDB_CORS_ALLOW_ORIGIN").ok(),
        request_rate_limit_per_minute,
        audit_log_enabled: audit_log_enabled_from_env() || audit_log_path.is_some(),
        audit_log_path,
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
    let limit = raw
        .parse::<u64>()
        .map_err(|_| "CORTEXDB_RATE_LIMIT_PER_MINUTE must be a positive integer".to_owned())?;
    if limit == 0 {
        return Err("CORTEXDB_RATE_LIMIT_PER_MINUTE must be greater than zero".to_owned());
    }
    Ok(limit)
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
