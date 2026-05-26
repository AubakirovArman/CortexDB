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
