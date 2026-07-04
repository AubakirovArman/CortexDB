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

#[path = "main/config_env.rs"]
mod config_env;
use config_env::*;

#[cfg(test)]
#[path = "main/tests.rs"]
mod tests;
