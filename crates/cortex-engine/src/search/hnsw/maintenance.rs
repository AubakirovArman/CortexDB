use super::super::hnsw_policy::{HnswMaintenancePolicy, HnswMaintenanceReport, HnswRebuildPolicy};
use super::HnswIndex;

impl HnswIndex {
    pub fn rebuild_if_needed(&mut self, policy: HnswRebuildPolicy) -> bool {
        if !self.should_rebuild(policy) {
            return false;
        }
        self.vectors
            .retain(|candidate, _| !self.deleted.contains(candidate));
        self.deleted.clear();
        self.rebuild_links();
        self.rebuild_count += 1;
        true
    }

    pub fn apply_maintenance(&mut self, policy: HnswMaintenancePolicy) -> HnswMaintenanceReport {
        let vectors_before = self.vectors.len();
        let deleted_before = self.deleted.len();
        let rebuilt = deleted_before >= policy.min_deleted_vectors
            && self.rebuild_if_needed(policy.rebuild_policy);
        HnswMaintenanceReport {
            vectors_before,
            deleted_before,
            rebuilt,
        }
    }

    pub fn maintenance_due(&self, policy: HnswMaintenancePolicy) -> bool {
        self.deleted.len() >= policy.min_deleted_vectors
            && self.should_rebuild(policy.rebuild_policy)
    }

    fn should_rebuild(&self, policy: HnswRebuildPolicy) -> bool {
        if self.deleted.is_empty() || self.vectors.is_empty() {
            return false;
        }
        let deleted_q16 = (self.deleted.len() as u64 * 65_535) / self.vectors.len() as u64;
        deleted_q16 >= u64::from(policy.deleted_fraction_q16)
    }

    fn rebuild_links(&mut self) {
        let vectors = self.vectors.clone();
        let deleted = self.deleted.clone();
        self.links.clear();
        self.upper_links.clear();
        self.vectors.clear();
        for (cell_id, vector) in vectors {
            if !deleted.contains(&cell_id) {
                let _ = self.add_vector(cell_id, vector);
            }
        }
    }
}
