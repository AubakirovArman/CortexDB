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
pub enum EngineFeature {
    ExperimentalHnsw,
    ExperimentalReplication,
    Dashboard,
}

impl EngineFeature {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ExperimentalHnsw => "experimental_hnsw",
            Self::ExperimentalReplication => "experimental_replication",
            Self::Dashboard => "dashboard",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EngineFeatureFlags {
    pub experimental_hnsw: bool,
    pub experimental_replication: bool,
    pub dashboard: bool,
}

impl EngineFeatureFlags {
    pub const fn production_safe() -> Self {
        Self {
            experimental_hnsw: false,
            experimental_replication: false,
            dashboard: false,
        }
    }

    pub const fn experimental_all() -> Self {
        Self {
            experimental_hnsw: true,
            experimental_replication: true,
            dashboard: true,
        }
    }

    pub const fn with_experimental_hnsw(mut self, enabled: bool) -> Self {
        self.experimental_hnsw = enabled;
        self
    }

    pub const fn with_experimental_replication(mut self, enabled: bool) -> Self {
        self.experimental_replication = enabled;
        self
    }

    pub const fn with_dashboard(mut self, enabled: bool) -> Self {
        self.dashboard = enabled;
        self
    }

    pub fn is_enabled(self, feature: EngineFeature) -> bool {
        match feature {
            EngineFeature::ExperimentalHnsw => self.experimental_hnsw,
            EngineFeature::ExperimentalReplication => self.experimental_replication,
            EngineFeature::Dashboard => self.dashboard,
        }
    }
}

impl Default for EngineFeatureFlags {
    fn default() -> Self {
        Self::production_safe()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DatabaseOptions {
    pub durability_mode: DurabilityMode,
    pub recovery_mode: RecoveryMode,
    pub stale_lock_policy: StaleLockPolicy,
    pub hnsw_build_config: HnswBuildConfig,
    pub feature_flags: EngineFeatureFlags,
}

impl Default for DatabaseOptions {
    fn default() -> Self {
        Self {
            durability_mode: DurabilityMode::Strict,
            recovery_mode: RecoveryMode::Strict,
            stale_lock_policy: StaleLockPolicy::Reject,
            hnsw_build_config: HnswBuildConfig::default(),
            feature_flags: EngineFeatureFlags::production_safe(),
        }
    }
}
