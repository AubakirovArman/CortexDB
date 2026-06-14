use cortex_storage::wal::DurabilityMode;

use crate::ingestion::IngestionBackpressurePolicy;
use crate::search::{HnswBuildConfig, TextAnalyzerConfig};

/// Thresholds and limits that govern when the background compactor selects a
/// subset of live segments for an incremental merge.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CompactionPolicy {
    /// Q16 pressure ratio at which compaction is considered necessary.
    pub trigger_pressure_q16: u32,
    /// Number of live segments that also triggers compaction.
    pub trigger_segment_count: usize,
    /// Maximum number of input segments merged in one incremental compaction.
    pub max_input_segments: usize,
    /// Maximum total input bytes merged in one incremental compaction.
    pub max_input_bytes: u64,
}

impl CompactionPolicy {
    pub const DEFAULT_TRIGGER_PRESSURE_Q16: u32 = 32_768; // 0.5
    pub const DEFAULT_TRIGGER_SEGMENT_COUNT: usize = 6;
    pub const DEFAULT_MAX_INPUT_SEGMENTS: usize = 4;
    pub const DEFAULT_MAX_INPUT_BYTES: u64 = 64 * 1024 * 1024;
}

impl Default for CompactionPolicy {
    fn default() -> Self {
        Self {
            trigger_pressure_q16: Self::DEFAULT_TRIGGER_PRESSURE_Q16,
            trigger_segment_count: Self::DEFAULT_TRIGGER_SEGMENT_COUNT,
            max_input_segments: Self::DEFAULT_MAX_INPUT_SEGMENTS,
            max_input_bytes: Self::DEFAULT_MAX_INPUT_BYTES,
        }
    }
}

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
pub enum PayloadResidency {
    Memory,
    Lazy,
}

pub const DEFAULT_PAYLOAD_CACHE_BYTES: usize = 64 * 1024 * 1024;
pub const DEFAULT_AQL_QUERY_CACHE_MAX_ENTRIES: usize = 128;

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
    pub payload_residency: PayloadResidency,
    pub payload_cache_bytes: usize,
    pub aql_query_cache_max_entries: usize,
    pub rebuild_lazy_payload_indexes_on_open: bool,
    pub hnsw_build_config: HnswBuildConfig,
    pub feature_flags: EngineFeatureFlags,
    pub ingestion_backpressure: IngestionBackpressurePolicy,
    pub compaction_policy: CompactionPolicy,
    pub text_analyzer: TextAnalyzerConfig,
}

impl Default for DatabaseOptions {
    fn default() -> Self {
        Self {
            durability_mode: DurabilityMode::Strict,
            recovery_mode: RecoveryMode::Strict,
            stale_lock_policy: StaleLockPolicy::Reject,
            payload_residency: PayloadResidency::Memory,
            payload_cache_bytes: DEFAULT_PAYLOAD_CACHE_BYTES,
            aql_query_cache_max_entries: DEFAULT_AQL_QUERY_CACHE_MAX_ENTRIES,
            rebuild_lazy_payload_indexes_on_open: true,
            hnsw_build_config: HnswBuildConfig::default(),
            feature_flags: EngineFeatureFlags::production_safe(),
            ingestion_backpressure: IngestionBackpressurePolicy::default(),
            compaction_policy: CompactionPolicy::default(),
            text_analyzer: TextAnalyzerConfig::default(),
        }
    }
}
