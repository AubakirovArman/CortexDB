use std::collections::BTreeMap;

use cortex_engine::HnswIndex;
use cortex_storage::hnsw::HnswGraphIndex;

#[test]
fn hnsw_graph_persistence_preserves_search_path() {
    let dir = tempfile::tempdir().unwrap();
    let graph_path = dir.path().join("segment-1.ach");
    let mut index = HnswIndex::new(2, 8);
    index.add_vector(1, vec![5, 0]);
    index.add_vector(2, vec![0, 5]);
    index.add_vector(3, vec![4, 0]);

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
