use std::collections::BTreeSet;

use super::*;
use crate::search::hnsw::VectorCollectionConfig;

#[test]
fn multi_layer_search_is_deterministic_for_same_query() {
    let mut index = HnswIndex::new_multilayer(4, 4, 3);
    index.set_config(VectorCollectionConfig {
        dimension: 2,
        metric: DistanceMetric::DotProduct,
    });
    index.add_vector(1, vec![10, 0]).unwrap();
    index.add_vector(2, vec![8, 2]).unwrap();
    index.add_vector(3, vec![0, 10]).unwrap();
    index.add_vector(4, vec![1, 9]).unwrap();

    let allowed = BTreeSet::from([1, 2, 3, 4]);
    let first = index.search_allowed(&[10, 0], &allowed, 3);
    let second = index.search_allowed(&[10, 0], &allowed, 3);

    assert_eq!(first, second);
    assert!(!first.is_empty());
}

#[test]
fn deleted_entry_and_search_nodes_are_skipped() {
    let mut index = HnswIndex::new_multilayer(4, 4, 2);
    index.set_config(VectorCollectionConfig {
        dimension: 2,
        metric: DistanceMetric::DotProduct,
    });
    index.add_vector(1, vec![10, 0]).unwrap();
    index.add_vector(2, vec![0, 10]).unwrap();
    index.remove_vector(2);

    let allowed = BTreeSet::from([1, 2]);
    let results = index.search_allowed(&[9, 1], &allowed, 2);

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].cell_id, 1);
}

#[test]
fn visited_budget_counts_entrypoint_and_base_search_candidates() {
    let mut index = HnswIndex::new_multilayer(8, 64, 3);
    index.set_config(VectorCollectionConfig {
        dimension: 2,
        metric: DistanceMetric::DotProduct,
    });
    index.add_vector(1, vec![10, 0]).unwrap();
    index.add_vector(2, vec![0, 10]).unwrap();
    index.add_vector(3, vec![9, 1]).unwrap();
    index.add_vector(4, vec![1, 9]).unwrap();
    let allowed = BTreeSet::from([1, 2, 3, 4]);

    let (_results_low, visited_low, exceeded_low) =
        index.search_allowed_with_budget(&[9, 1], &allowed, 2, Some(1));
    assert!(exceeded_low);
    assert!(visited_low >= 1);

    let (_results_ok, visited_ok, exceeded_ok) =
        index.search_allowed_with_budget(&[9, 1], &allowed, 2, Some(5));
    assert!(!exceeded_ok);
    assert!(visited_ok > visited_low);
}
