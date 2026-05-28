use std::collections::{BTreeMap, BTreeSet};

use cortex_engine::{HnswIndex, HnswMaintenancePolicy, HnswRebuildPolicy};
use cortex_storage::hnsw::HnswGraphIndex;

#[test]
fn hnsw_graph_persistence_preserves_search_path() {
    let dir = tempfile::tempdir().unwrap();
    let graph_path = dir.path().join("segment-1.ach");
    let mut index = HnswIndex::new(2, 8);
    let _ = index.add_vector(1, vec![5, 0]);
    let _ = index.add_vector(2, vec![0, 5]);
    let _ = index.add_vector(3, vec![4, 0]);

    index.graph_index().write(&graph_path).unwrap();
    let graph = HnswGraphIndex::read(&graph_path).unwrap();
    let restored = HnswIndex::from_graph(
        BTreeMap::from([(1, vec![5, 0]), (2, vec![0, 5]), (3, vec![4, 0])]),
        graph,
        2,
        8,
    );

    assert_eq!(restored.search(&[5, 0], 1)[0].cell_id, 1);
}

#[test]
fn hnsw_search_allowed_filters_runtime_scope_mask() {
    let mut index = HnswIndex::new(2, 8);
    let _ = index.add_vector(1, vec![10, 0]);
    let _ = index.add_vector(2, vec![9, 0]);
    let _ = index.add_vector(3, vec![0, 10]);

    let results = index.search_allowed(&[10, 0], &BTreeSet::from([2]), 10);

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].cell_id, 2);
}

#[test]
fn hnsw_delete_and_rebuild_policy_removes_deleted_vectors() {
    let mut index = HnswIndex::new_multilayer(2, 8, 3);
    let _ = index.add_vector(1, vec![10, 0]);
    let _ = index.add_vector(2, vec![9, 0]);
    let _ = index.add_vector(3, vec![0, 10]);

    assert_eq!(index.layer_count(), 3);
    assert!(index.remove_vector(1));
    assert!(index.rebuild_if_needed(HnswRebuildPolicy {
        deleted_fraction_q16: 1,
    }));

    assert_ne!(index.search(&[10, 0], 1)[0].cell_id, 1);
}

#[test]
fn hnsw_maintenance_reports_rebuild_lifecycle() {
    let mut index = HnswIndex::new(2, 8);
    let _ = index.add_vector(1, vec![10, 0]);
    let _ = index.add_vector(2, vec![9, 0]);
    let _ = index.add_vector(3, vec![0, 10]);
    assert!(index.remove_vector(2));
    let policy = HnswMaintenancePolicy {
        rebuild_policy: HnswRebuildPolicy {
            deleted_fraction_q16: 1,
        },
        min_deleted_vectors: 1,
    };

    assert!(index.maintenance_due(policy));
    let report = index.apply_maintenance(policy);

    assert!(report.rebuilt);
    assert_eq!(report.vectors_before, 3);
    assert_eq!(report.deleted_before, 1);
    assert_eq!(index.deleted_count(), 0);
    assert_eq!(index.vector_count(), 2);
}

#[test]
fn hnsw_integrity_report_catches_structural_link_errors() {
    let index = HnswIndex::from_graph(
        BTreeMap::from([(1, vec![10, 0]), (2, vec![0, 10])]),
        HnswGraphIndex {
            links: BTreeMap::from([(1, BTreeSet::from([1, 999]))]),
            dimension: 2,
            metric: 0,
        },
        2,
        8,
    );

    let report = index.integrity_report();

    assert!(!report.is_valid());
    assert_eq!(report.self_links, 1);
    assert_eq!(report.missing_neighbor_links, 1);
    assert!(report.summary().contains("missing_neighbor_links=1"));
}
