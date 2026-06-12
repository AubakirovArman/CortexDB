use cortex_core::CellId;
use cortex_engine::{Database, EngineConfig, EngineError};

pub(crate) fn fmt_engine_error(e: EngineError) -> String {
    let message = e.to_string();
    match e.cli_hint() {
        Some(hint) => format!("{message}\n  -> {hint}"),
        None => message,
    }
}

pub(crate) fn open_database(path: &str, experimental_hnsw: bool) -> Result<Database, String> {
    let mut options = EngineConfig::from_env()
        .map_err(|error| error.to_string())?
        .database_options();
    options.feature_flags = options
        .feature_flags
        .with_experimental_hnsw(options.feature_flags.experimental_hnsw || experimental_hnsw);
    Database::open_with_options(path, options).map_err(fmt_engine_error)
}

pub(crate) fn validate_tenant_id(tenant: &str) -> bool {
    crate::cli_doctor::is_valid_tenant_arg(tenant)
}

pub(super) fn parse_cell_id(value: &str) -> Result<CellId, String> {
    value.parse::<u64>().map(CellId).map_err(|_| {
        format!(
            "cell_id must be a positive integer, got: {value:?}\n  → example: cortexdb get ./db 42"
        )
    })
}
