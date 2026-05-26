use std::collections::BTreeMap;

use cortex_storage::vectors::VectorIndex;

#[test]
fn vector_dimension_report_accepts_consistent_vectors() {
    let index = VectorIndex {
        vectors: BTreeMap::from([(1, vec![1, 2]), (2, vec![3, 4])]),
    };

    let report = index.dimension_report();

    assert!(report.is_valid());
    assert_eq!(report.vector_count, 2);
    assert_eq!(report.expected_dimension, Some(2));
    assert_eq!(report.mismatched_vectors, 0);
}

#[test]
fn vector_dimension_report_rejects_mixed_dimensions() {
    let index = VectorIndex {
        vectors: BTreeMap::from([(1, vec![1, 2]), (2, vec![3])]),
    };

    let report = index.dimension_report();

    assert!(!report.is_valid());
    assert_eq!(report.mismatched_vectors, 1);
    assert!(report.summary().contains("mismatched_vectors=1"));
}
