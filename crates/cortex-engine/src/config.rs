use std::collections::BTreeMap;
use std::env;
use std::fmt;

use cortex_storage::wal::DurabilityMode;

use crate::options::{
    CompactionPolicy, DatabaseOptions, EngineFeatureFlags, PayloadResidency, RecoveryMode,
    StaleLockPolicy, TieredStorageCompressionPolicy, TieredStorageOptions,
    DEFAULT_AQL_QUERY_CACHE_MAX_ENTRIES, DEFAULT_PAYLOAD_CACHE_BYTES,
    DEFAULT_WAL_ARCHIVE_MAX_FILES,
};
use crate::search::{HnswBuildConfig, HnswBuildProfile};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct EngineConfig {
    pub database_options: DatabaseOptions,
}

impl EngineConfig {
    pub fn from_env() -> Result<Self, EngineConfigError> {
        Self::from_env_vars(env::vars())
    }

    pub fn from_env_vars<I, K, V>(vars: I) -> Result<Self, EngineConfigError>
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<String>,
    {
        let vars = vars
            .into_iter()
            .map(|(key, value)| (key.into(), value.into()))
            .collect::<BTreeMap<_, _>>();
        let feature_flags = EngineFeatureFlags::production_safe()
            .with_experimental_hnsw(parse_bool_var(&vars, "CORTEXDB_EXPERIMENTAL_HNSW", false)?)
            .with_experimental_replication(parse_bool_var(
                &vars,
                "CORTEXDB_EXPERIMENTAL_REPLICATION",
                false,
            )?)
            .with_dashboard(parse_bool_var(&vars, "CORTEXDB_DASHBOARD", false)?);

        let mut database_options = DatabaseOptions {
            durability_mode: parse_durability_mode(&vars)?,
            recovery_mode: parse_recovery_mode(&vars)?,
            stale_lock_policy: parse_stale_lock_policy(&vars)?,
            payload_residency: parse_payload_residency(&vars)?,
            payload_cache_bytes: parse_usize_var(
                &vars,
                "CORTEXDB_PAYLOAD_CACHE_BYTES",
                DEFAULT_PAYLOAD_CACHE_BYTES,
            )?,
            tiered_storage: parse_tiered_storage_options(&vars)?,
            aql_query_cache_max_entries: parse_usize_var(
                &vars,
                "CORTEXDB_AQL_QUERY_CACHE_MAX_ENTRIES",
                DEFAULT_AQL_QUERY_CACHE_MAX_ENTRIES,
            )?,
            wal_archive_enabled: parse_bool_var(&vars, "CORTEXDB_WAL_ARCHIVE", false)?,
            wal_archive_max_files: parse_usize_var(
                &vars,
                "CORTEXDB_WAL_ARCHIVE_MAX_FILES",
                DEFAULT_WAL_ARCHIVE_MAX_FILES,
            )?
            .max(1),
            rebuild_lazy_payload_indexes_on_open: true,
            hnsw_build_config: parse_hnsw_build_config(&vars)?,
            feature_flags,
            ingestion_backpressure: Default::default(),
            compaction_policy: CompactionPolicy::default(),
            text_analyzer: Default::default(),
        };
        database_options.hnsw_build_config = database_options.hnsw_build_config.normalized();
        Ok(Self { database_options })
    }

    pub fn database_options(self) -> DatabaseOptions {
        self.database_options
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EngineConfigError {
    pub variable: &'static str,
    pub value: String,
    pub expected: &'static str,
}

impl EngineConfigError {
    fn new(variable: &'static str, value: &str, expected: &'static str) -> Self {
        Self {
            variable,
            value: value.to_owned(),
            expected,
        }
    }
}

impl fmt::Display for EngineConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "invalid {}={:?}; expected {}",
            self.variable, self.value, self.expected
        )
    }
}

impl std::error::Error for EngineConfigError {}

fn var<'a>(vars: &'a BTreeMap<String, String>, name: &'static str) -> Option<&'a str> {
    vars.get(name).map(String::as_str)
}

fn parse_bool_var(
    vars: &BTreeMap<String, String>,
    name: &'static str,
    default: bool,
) -> Result<bool, EngineConfigError> {
    let Some(raw) = var(vars, name) else {
        return Ok(default);
    };
    match raw.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Ok(true),
        "0" | "false" | "no" | "off" => Ok(false),
        _ => Err(EngineConfigError::new(
            name,
            raw,
            "one of true,false,1,0,yes,no,on,off",
        )),
    }
}

fn parse_durability_mode(
    vars: &BTreeMap<String, String>,
) -> Result<DurabilityMode, EngineConfigError> {
    let Some(raw) = var(vars, "CORTEXDB_DURABILITY_MODE") else {
        return Ok(DurabilityMode::Strict);
    };
    match raw.trim().to_ascii_lowercase().as_str() {
        "strict" => Ok(DurabilityMode::Strict),
        "balanced" => Ok(DurabilityMode::Balanced),
        _ => Err(EngineConfigError::new(
            "CORTEXDB_DURABILITY_MODE",
            raw,
            "strict or balanced",
        )),
    }
}

fn parse_recovery_mode(vars: &BTreeMap<String, String>) -> Result<RecoveryMode, EngineConfigError> {
    let Some(raw) = var(vars, "CORTEXDB_RECOVERY_MODE") else {
        return Ok(RecoveryMode::Strict);
    };
    match raw.trim().to_ascii_lowercase().as_str() {
        "strict" => Ok(RecoveryMode::Strict),
        "best_effort" | "best-effort" | "besteffort" => Ok(RecoveryMode::BestEffort),
        _ => Err(EngineConfigError::new(
            "CORTEXDB_RECOVERY_MODE",
            raw,
            "strict or best_effort",
        )),
    }
}

fn parse_stale_lock_policy(
    vars: &BTreeMap<String, String>,
) -> Result<StaleLockPolicy, EngineConfigError> {
    let Some(raw) = var(vars, "CORTEXDB_STALE_LOCK_POLICY") else {
        return Ok(StaleLockPolicy::Reject);
    };
    match raw.trim().to_ascii_lowercase().as_str() {
        "reject" => Ok(StaleLockPolicy::Reject),
        "break" => Ok(StaleLockPolicy::Break),
        _ => Err(EngineConfigError::new(
            "CORTEXDB_STALE_LOCK_POLICY",
            raw,
            "reject or break",
        )),
    }
}

fn parse_payload_residency(
    vars: &BTreeMap<String, String>,
) -> Result<PayloadResidency, EngineConfigError> {
    let Some(raw) = var(vars, "CORTEXDB_PAYLOAD_RESIDENCY") else {
        return Ok(PayloadResidency::Memory);
    };
    match raw.trim().to_ascii_lowercase().as_str() {
        "memory" => Ok(PayloadResidency::Memory),
        "lazy" => Ok(PayloadResidency::Lazy),
        _ => Err(EngineConfigError::new(
            "CORTEXDB_PAYLOAD_RESIDENCY",
            raw,
            "memory or lazy",
        )),
    }
}

fn parse_tiered_storage_options(
    vars: &BTreeMap<String, String>,
) -> Result<TieredStorageOptions, EngineConfigError> {
    Ok(TieredStorageOptions {
        enabled: parse_bool_var(vars, "CORTEXDB_TIERED_STORAGE_V2", false)?,
        compression_policy: parse_tiered_storage_compression_policy(vars)?,
    })
}

fn parse_tiered_storage_compression_policy(
    vars: &BTreeMap<String, String>,
) -> Result<TieredStorageCompressionPolicy, EngineConfigError> {
    let Some(raw) = var(vars, "CORTEXDB_TIERED_STORAGE_COMPRESSION") else {
        return Ok(TieredStorageCompressionPolicy::None);
    };
    match raw.trim().to_ascii_lowercase().as_str() {
        "none" => Ok(TieredStorageCompressionPolicy::None),
        "zstd_reserved" | "zstd-reserved" | "zstd" => {
            Ok(TieredStorageCompressionPolicy::ZstdReserved)
        }
        _ => Err(EngineConfigError::new(
            "CORTEXDB_TIERED_STORAGE_COMPRESSION",
            raw,
            "none or zstd_reserved",
        )),
    }
}

fn parse_usize_var(
    vars: &BTreeMap<String, String>,
    name: &'static str,
    default: usize,
) -> Result<usize, EngineConfigError> {
    let Some(raw) = var(vars, name) else {
        return Ok(default);
    };
    raw.trim()
        .parse::<usize>()
        .map_err(|_| EngineConfigError::new(name, raw, "a non-negative integer"))
}

fn parse_hnsw_build_config(
    vars: &BTreeMap<String, String>,
) -> Result<HnswBuildConfig, EngineConfigError> {
    let Some(raw) = var(vars, "CORTEXDB_HNSW_PROFILE") else {
        return Ok(HnswBuildConfig::default());
    };
    let profile = match raw.trim().to_ascii_lowercase().as_str() {
        "fast" => HnswBuildProfile::Fast,
        "balanced" => HnswBuildProfile::Balanced,
        "semantic" => HnswBuildProfile::Semantic,
        "audit" => HnswBuildProfile::Audit,
        _ => {
            return Err(EngineConfigError::new(
                "CORTEXDB_HNSW_PROFILE",
                raw,
                "fast, balanced, semantic, or audit",
            ))
        }
    };
    Ok(HnswBuildConfig::for_profile(profile))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn engine_config_defaults_to_database_options_defaults() {
        assert_eq!(
            EngineConfig::from_env_vars(Vec::<(String, String)>::new())
                .unwrap()
                .database_options,
            DatabaseOptions::default()
        );
    }

    #[test]
    fn engine_config_parses_database_options_from_env_vars() {
        let config = EngineConfig::from_env_vars([
            ("CORTEXDB_DURABILITY_MODE", "balanced"),
            ("CORTEXDB_RECOVERY_MODE", "best_effort"),
            ("CORTEXDB_STALE_LOCK_POLICY", "break"),
            ("CORTEXDB_PAYLOAD_RESIDENCY", "lazy"),
            ("CORTEXDB_PAYLOAD_CACHE_BYTES", "4096"),
            ("CORTEXDB_TIERED_STORAGE_V2", "true"),
            ("CORTEXDB_TIERED_STORAGE_COMPRESSION", "zstd_reserved"),
            ("CORTEXDB_AQL_QUERY_CACHE_MAX_ENTRIES", "64"),
            ("CORTEXDB_WAL_ARCHIVE", "true"),
            ("CORTEXDB_WAL_ARCHIVE_MAX_FILES", "8"),
            ("CORTEXDB_HNSW_PROFILE", "audit"),
            ("CORTEXDB_EXPERIMENTAL_HNSW", "true"),
            ("CORTEXDB_EXPERIMENTAL_REPLICATION", "1"),
            ("CORTEXDB_DASHBOARD", "yes"),
        ])
        .unwrap();
        assert_eq!(
            config.database_options.durability_mode,
            DurabilityMode::Balanced
        );
        assert_eq!(
            config.database_options.recovery_mode,
            RecoveryMode::BestEffort
        );
        assert_eq!(
            config.database_options.stale_lock_policy,
            StaleLockPolicy::Break
        );
        assert_eq!(
            config.database_options.payload_residency,
            PayloadResidency::Lazy
        );
        assert_eq!(config.database_options.payload_cache_bytes, 4096);
        assert!(config.database_options.tiered_storage.enabled);
        assert_eq!(
            config.database_options.tiered_storage.compression_policy,
            TieredStorageCompressionPolicy::ZstdReserved
        );
        assert_eq!(config.database_options.aql_query_cache_max_entries, 64);
        assert!(config.database_options.wal_archive_enabled);
        assert_eq!(config.database_options.wal_archive_max_files, 8);
        assert_eq!(
            config.database_options.hnsw_build_config,
            HnswBuildConfig::for_profile(HnswBuildProfile::Audit)
        );
        assert!(config.database_options.feature_flags.experimental_hnsw);
        assert!(
            config
                .database_options
                .feature_flags
                .experimental_replication
        );
        assert!(config.database_options.feature_flags.dashboard);
    }

    #[test]
    fn engine_config_rejects_invalid_values() {
        let error = EngineConfig::from_env_vars([("CORTEXDB_RECOVERY_MODE", "maybe")]).unwrap_err();
        assert_eq!(error.variable, "CORTEXDB_RECOVERY_MODE");
        let error =
            EngineConfig::from_env_vars([("CORTEXDB_PAYLOAD_RESIDENCY", "maybe")]).unwrap_err();
        assert_eq!(error.variable, "CORTEXDB_PAYLOAD_RESIDENCY");
        let error =
            EngineConfig::from_env_vars([("CORTEXDB_PAYLOAD_CACHE_BYTES", "maybe")]).unwrap_err();
        assert_eq!(error.variable, "CORTEXDB_PAYLOAD_CACHE_BYTES");
        let error = EngineConfig::from_env_vars([("CORTEXDB_TIERED_STORAGE_COMPRESSION", "maybe")])
            .unwrap_err();
        assert_eq!(error.variable, "CORTEXDB_TIERED_STORAGE_COMPRESSION");
        let error =
            EngineConfig::from_env_vars([("CORTEXDB_AQL_QUERY_CACHE_MAX_ENTRIES", "maybe")])
                .unwrap_err();
        assert_eq!(error.variable, "CORTEXDB_AQL_QUERY_CACHE_MAX_ENTRIES");
    }
}
