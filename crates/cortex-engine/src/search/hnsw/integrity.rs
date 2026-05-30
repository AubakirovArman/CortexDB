use super::HnswIndex;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HnswIntegrityReport {
    pub vector_count: usize,
    pub link_nodes: usize,
    pub zero_ids: usize,
    pub missing_link_nodes: usize,
    pub missing_neighbor_links: usize,
    pub self_links: usize,
    pub deleted_link_references: usize,
    pub upper_layers: usize,
    pub upper_link_nodes: usize,
}

impl HnswIntegrityReport {
    pub fn is_valid(self) -> bool {
        self.zero_ids == 0
            && self.missing_link_nodes == 0
            && self.missing_neighbor_links == 0
            && self.self_links == 0
            && self.deleted_link_references == 0
    }

    pub fn summary(self) -> String {
        format!(
            "vectors={} link_nodes={} upper_layers={} upper_link_nodes={} zero_ids={} missing_link_nodes={} missing_neighbor_links={} self_links={} deleted_link_references={}",
            self.vector_count,
            self.link_nodes,
            self.upper_layers,
            self.upper_link_nodes,
            self.zero_ids,
            self.missing_link_nodes,
            self.missing_neighbor_links,
            self.self_links,
            self.deleted_link_references
        )
    }
}

impl HnswIndex {
    pub fn integrity_report(&self) -> HnswIntegrityReport {
        let mut report = HnswIntegrityReport {
            vector_count: self.vectors.len(),
            link_nodes: self.links.len(),
            upper_layers: self.upper_links.len(),
            upper_link_nodes: self.upper_links.values().map(|links| links.len()).sum(),
            zero_ids: self.vectors.keys().filter(|id| **id == 0).count(),
            ..HnswIntegrityReport::default()
        };
        for deleted in &self.deleted {
            if *deleted == 0 {
                report.zero_ids += 1;
            }
        }
        for (node, neighbors) in &self.links {
            inspect_links(self, node, neighbors, &mut report);
        }
        for links in self.upper_links.values() {
            for (node, neighbors) in links {
                inspect_links(self, node, neighbors, &mut report);
            }
        }
        report
    }

    pub fn verify_hnsw_integrity(&self) -> bool {
        self.integrity_report().is_valid()
    }
}

fn inspect_links(
    index: &HnswIndex,
    node: &u32,
    neighbors: &std::collections::BTreeSet<u32>,
    report: &mut HnswIntegrityReport,
) {
    if *node == 0 {
        report.zero_ids += 1;
    }
    if !index.vectors.contains_key(node) {
        report.missing_link_nodes += 1;
    }
    if index.deleted.contains(node) {
        report.deleted_link_references += 1;
    }
    for neighbor in neighbors {
        if *neighbor == 0 {
            report.zero_ids += 1;
        }
        if neighbor == node {
            report.self_links += 1;
        }
        if !index.vectors.contains_key(neighbor) {
            report.missing_neighbor_links += 1;
        }
        if index.deleted.contains(neighbor) {
            report.deleted_link_references += 1;
        }
    }
}
