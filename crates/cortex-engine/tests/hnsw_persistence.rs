use std::collections::{BTreeMap, BTreeSet};

use cortex_core::CellId;
use cortex_engine::{
    Database, DatabaseOptions, DistanceMetric, EngineFeatureFlags, HnswBuildConfig,
    HnswBuildProfile, HnswIndex, HnswMaintenancePolicy, HnswRebuildPolicy,
};
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
fn hnsw_multilayer_graph_persists_upper_layers() {
    let dir = tempfile::tempdir().unwrap();
    let graph_path = dir.path().join("segment-1.ach");
    let mut index = HnswIndex::new_multilayer(4, 16, 4);
    for id in 1..=64 {
        let _ = index.add_vector(id, vec![id as i16, (64 - id) as i16]);
    }

    let graph = index.graph_index();
    assert!(
        !graph.upper_layers.is_empty(),
        "deterministic multilayer builder should produce upper layers"
    );
    graph.write(&graph_path).unwrap();

    let restored_graph = HnswGraphIndex::read(&graph_path).unwrap();
    let restored = HnswIndex::from_graph(
        (1..=64)
            .map(|id| (id, vec![id as i16, (64 - id) as i16]))
            .collect(),
        restored_graph,
        4,
        16,
    );

    assert!(restored.layer_count() > 1);
    assert!(!restored.search(&[64, 0], 5).is_empty());
}

#[test]
fn hnsw_multilayer_search_respects_visit_budget() {
    let mut index = HnswIndex::new_multilayer(4, 16, 4);
    for id in 1..=64 {
        let _ = index.add_vector(id, vec![id as i16, (64 - id) as i16]);
    }

    let (_, visited, budget_exceeded) =
        index.search_allowed_with_budget(&[64, 0], &BTreeSet::from_iter(1..=64), 5, Some(1));

    assert!(budget_exceeded);
    assert!(visited >= 1);
}

#[test]
fn database_hnsw_build_config_controls_checkpoint_graph_density() {
    let sparse_edges = checkpoint_edge_count(HnswBuildConfig {
        max_neighbors: 1,
        ef_search: 8,
        layer_count: 1,
        ef_construction: 8,
        metric: DistanceMetric::DotProduct,
    });
    let semantic_edges =
        checkpoint_edge_count(HnswBuildConfig::for_profile(HnswBuildProfile::Semantic));

    assert!(
        semantic_edges > sparse_edges,
        "semantic profile should persist a denser checkpoint HNSW graph"
    );
}

#[test]
fn database_checkpoint_persists_hnsw_build_profile_metadata() {
    let dir = tempfile::tempdir().unwrap();
    let config = HnswBuildConfig::for_profile(HnswBuildProfile::Semantic);
    let mut db = Database::open_with_options(dir.path(), hnsw_options(config)).unwrap();
    db.put_cell(
        CellId(1),
        b"scope=project:investments\nstatus=ready\nvector=10,0\n\nalpha".to_vec(),
    )
    .unwrap();
    db.checkpoint().unwrap();

    let graph = HnswGraphIndex::read(dir.path().join("segments/segment-1.ach")).unwrap();

    assert_eq!(graph.max_neighbors, config.max_neighbors as u32);
    assert_eq!(graph.ef_search, config.ef_search as u32);
    assert_eq!(graph.layer_count, config.layer_count as u32);
    assert_eq!(graph.ef_construction, config.ef_construction as u32);
}

#[test]
fn hnsw_build_profiles_match_documented_slo_shapes() {
    let balanced = HnswBuildConfig::for_profile(HnswBuildProfile::Balanced);
    let audit = HnswBuildConfig::for_profile(HnswBuildProfile::Audit);

    assert_eq!(balanced.max_neighbors, 16);
    assert_eq!(balanced.ef_search, 128);
    assert_eq!(balanced.ef_construction, 128);
    assert_eq!(balanced.layer_count, 4);
    assert!(audit.max_neighbors > balanced.max_neighbors);
    assert!(audit.ef_search > balanced.ef_search);
    assert!(audit.ef_construction > balanced.ef_construction);
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
            ..HnswGraphIndex::default()
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

fn checkpoint_edge_count(config: HnswBuildConfig) -> usize {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open_with_options(dir.path(), hnsw_options(config)).unwrap();
    for id in 1..=64 {
        db.put_cell(
            CellId(id),
            format!(
                "scope=project:investments\nstatus=ready\nvector={},{}\n\ncell {id}",
                id,
                64 - id
            )
            .into_bytes(),
        )
        .unwrap();
    }
    db.checkpoint().unwrap();
    db.ann_metrics().total_edges
}

fn hnsw_options(config: HnswBuildConfig) -> DatabaseOptions {
    DatabaseOptions {
        hnsw_build_config: config,
        feature_flags: EngineFeatureFlags::production_safe().with_experimental_hnsw(true),
        ..DatabaseOptions::default()
    }
}
