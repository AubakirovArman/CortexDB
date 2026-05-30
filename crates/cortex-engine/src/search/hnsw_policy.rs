use super::hnsw::DistanceMetric;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HnswBuildProfile {
    Fast,
    Balanced,
    Semantic,
    Audit,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HnswBuildConfig {
    pub max_neighbors: usize,
    pub ef_search: usize,
    pub layer_count: usize,
    pub metric: DistanceMetric,
}

impl HnswBuildConfig {
    pub fn for_profile(profile: HnswBuildProfile) -> Self {
        match profile {
            HnswBuildProfile::Fast => Self {
                max_neighbors: 8,
                ef_search: 64,
                layer_count: 3,
                metric: DistanceMetric::DotProduct,
            },
            HnswBuildProfile::Balanced => Self {
                max_neighbors: 16,
                ef_search: 128,
                layer_count: 4,
                metric: DistanceMetric::DotProduct,
            },
            HnswBuildProfile::Semantic => Self {
                max_neighbors: 24,
                ef_search: 192,
                layer_count: 5,
                metric: DistanceMetric::DotProduct,
            },
            HnswBuildProfile::Audit => Self {
                max_neighbors: 32,
                ef_search: 256,
                layer_count: 5,
                metric: DistanceMetric::DotProduct,
            },
        }
    }

    pub fn normalized(self) -> Self {
        Self {
            max_neighbors: self.max_neighbors.max(1),
            ef_search: self.ef_search.max(1),
            layer_count: self.layer_count.max(1),
            metric: self.metric,
        }
    }
}

impl Default for HnswBuildConfig {
    fn default() -> Self {
        Self::for_profile(HnswBuildProfile::Balanced)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HnswRebuildPolicy {
    pub deleted_fraction_q16: u16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HnswMaintenancePolicy {
    pub rebuild_policy: HnswRebuildPolicy,
    pub min_deleted_vectors: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HnswMaintenanceReport {
    pub vectors_before: usize,
    pub deleted_before: usize,
    pub rebuilt: bool,
}

impl Default for HnswRebuildPolicy {
    fn default() -> Self {
        Self {
            deleted_fraction_q16: 16_384,
        }
    }
}

impl Default for HnswMaintenancePolicy {
    fn default() -> Self {
        Self {
            rebuild_policy: HnswRebuildPolicy::default(),
            min_deleted_vectors: 1,
        }
    }
}
