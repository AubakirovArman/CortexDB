use std::collections::{BTreeMap, BTreeSet};

use cortex_storage::hnsw::HnswGraphIndex;
use cortex_storage::StorageError;

#[test]
fn ach_hnsw_graph_roundtrips_links() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("segment-1.ach");
    let graph = HnswGraphIndex {
        links: BTreeMap::from([(1, BTreeSet::from([2, 3])), (2, BTreeSet::from([1]))]),
        dimension: 8,
        metric: 0,
    };

    graph.write(&path).unwrap();

    assert_eq!(HnswGraphIndex::read(&path).unwrap(), graph);
    assert!(!dir.path().join("segment-1.ach.tmp").exists());
}

#[test]
fn corrupt_hnsw_graph_is_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("segment-1.ach");
    HnswGraphIndex {
        links: BTreeMap::from([(1, BTreeSet::from([2]))]),
        dimension: 0,
        metric: 0,
    }
    .write(&path)
    .unwrap();
    let mut bytes = std::fs::read(&path).unwrap();
    *bytes.last_mut().unwrap() ^= 0xff;
    std::fs::write(&path, bytes).unwrap();

    assert!(matches!(
        HnswGraphIndex::read(&path).unwrap_err(),
        StorageError::InvalidHnswGraphFile
    ));
}
