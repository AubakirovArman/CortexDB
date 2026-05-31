use std::collections::{BTreeMap, BTreeSet};

use cortex_storage::format::HNSW_GRAPH_MAGIC;
use cortex_storage::hnsw::HnswGraphIndex;
use cortex_storage::wal::checksum::crc32c;
use cortex_storage::StorageError;

#[test]
fn ach_hnsw_graph_roundtrips_links() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("segment-1.ach");
    let graph = HnswGraphIndex {
        links: BTreeMap::from([(1, BTreeSet::from([2, 3])), (2, BTreeSet::from([1]))]),
        dimension: 8,
        metric: 0,
        ..HnswGraphIndex::default()
    };

    graph.write(&path).unwrap();

    assert_eq!(HnswGraphIndex::read(&path).unwrap(), graph);
    assert!(!dir.path().join("segment-1.ach.tmp").exists());
}

#[test]
fn ach_hnsw_graph_roundtrips_upper_layers() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("segment-1.ach");
    let graph = HnswGraphIndex {
        links: BTreeMap::from([(1, BTreeSet::from([2])), (2, BTreeSet::from([1]))]),
        dimension: 2,
        metric: 0,
        upper_layers: BTreeMap::from([(1, BTreeMap::from([(1, BTreeSet::from([2]))]))]),
        max_neighbors: 16,
        ef_search: 128,
        layer_count: 4,
        ef_construction: 256,
    };

    graph.write(&path).unwrap();

    let decoded = HnswGraphIndex::read(&path).unwrap();
    assert_eq!(decoded, graph);
    assert_eq!(decoded.upper_layers[&1][&1], BTreeSet::from([2]));
    assert_eq!(decoded.max_neighbors, 16);
    assert_eq!(decoded.ef_search, 128);
    assert_eq!(decoded.layer_count, 4);
    assert_eq!(decoded.ef_construction, 256);
}

#[test]
fn ach_hnsw_graph_reads_legacy_without_upper_layers() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("legacy.ach");
    let mut bytes = Vec::from(&HNSW_GRAPH_MAGIC[..]);
    put_u32(&mut bytes, 1);
    put_u32(&mut bytes, 1);
    put_u32(&mut bytes, 1);
    put_u32(&mut bytes, 2);
    put_u32(&mut bytes, 2);
    put_u32(&mut bytes, 0);
    let checksum = crc32c(&bytes);
    bytes.extend_from_slice(&checksum.to_le_bytes());
    std::fs::write(&path, bytes).unwrap();

    let graph = HnswGraphIndex::read(&path).unwrap();

    assert_eq!(graph.dimension, 2);
    assert_eq!(graph.links[&1], BTreeSet::from([2]));
    assert!(graph.upper_layers.is_empty());
}

#[test]
fn corrupt_hnsw_graph_is_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("segment-1.ach");
    HnswGraphIndex {
        links: BTreeMap::from([(1, BTreeSet::from([2]))]),
        dimension: 0,
        metric: 0,
        ..HnswGraphIndex::default()
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

fn put_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}
