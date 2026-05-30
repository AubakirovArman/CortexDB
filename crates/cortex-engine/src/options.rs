use cortex_storage::wal::DurabilityMode;

use crate::search::HnswBuildConfig;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecoveryMode {
    Strict,
    BestEffort,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StaleLockPolicy {
    Reject,
    Break,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DatabaseOptions {
    pub durability_mode: DurabilityMode,
    pub recovery_mode: RecoveryMode,
    pub stale_lock_policy: StaleLockPolicy,
    pub hnsw_build_config: HnswBuildConfig,
}

impl Default for DatabaseOptions {
    fn default() -> Self {
        Self {
            durability_mode: DurabilityMode::Strict,
            recovery_mode: RecoveryMode::Strict,
            stale_lock_policy: StaleLockPolicy::Reject,
            hnsw_build_config: HnswBuildConfig::default(),
        }
    }
}
