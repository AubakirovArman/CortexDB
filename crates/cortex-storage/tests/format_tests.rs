use std::collections::{BTreeMap, BTreeSet};

use cortex_storage::format::{
    storage_format_specs, StorageFormatKind, HNSW_GRAPH_MAGIC, LEGACY_LEXICAL_INDEX_MAGIC,
    LEGACY_LEXICAL_INDEX_V1_MAGIC, LEXICAL_INDEX_MAGIC,
};
use cortex_storage::hnsw::HnswGraphIndex;
use cortex_storage::indexes::{BitmapIndex, LexicalIndex};
use cortex_storage::manifest::StorageManifest;
use cortex_storage::segment::{SegmentCell, SegmentWriter};
use cortex_storage::vectors::VectorIndex;
use cortex_storage::wal::{DurabilityMode, WalWriter, ACLOG_MAGIC};

#[test]
fn storage_format_inventory_lists_current_core_formats() {
    let specs = storage_format_specs();
    assert_eq!(specs.len(), 7);
    assert_eq!(specs[0].kind, StorageFormatKind::AclogWal);
    assert_eq!(specs[0].current_magic, ACLOG_MAGIC);
    assert_eq!(specs[3].kind, StorageFormatKind::LexicalIndex);
    assert_eq!(specs[3].current_magic, LEXICAL_INDEX_MAGIC);
    assert_eq!(
        specs[3].legacy_magics,
        &[&LEGACY_LEXICAL_INDEX_MAGIC, &LEGACY_LEXICAL_INDEX_V1_MAGIC]
    );
    assert_eq!(specs[4].kind, StorageFormatKind::VectorIndex);
    assert_eq!(specs[5].kind, StorageFormatKind::HnswGraph);
    assert_eq!(specs[5].current_magic, HNSW_GRAPH_MAGIC);
}

#[test]
fn written_storage_files_match_current_format_inventory() {
    let dir = tempfile::tempdir().unwrap();
    let wal = dir.path().join("db.aclog");
    let segment = dir.path().join("s.acs");
    let bitmap = dir.path().join("s.acb");
    let lexical = dir.path().join("s.aci");
    let vector = dir.path().join("s.acv");
    let hnsw = dir.path().join("s.ach");
    let manifest = dir.path().join("manifest.acm");

    let writer = WalWriter::start(&wal, DurabilityMode::Strict).unwrap();
    writer.shutdown().unwrap();
    SegmentWriter::write(&segment, &[cell()]).unwrap();
    BitmapIndex {
        bitmaps: BTreeMap::from([(1, BTreeSet::from([1]))]),
    }
    .write(&bitmap)
    .unwrap();
    LexicalIndex {
        terms: BTreeMap::from([("budget".to_owned(), BTreeSet::from([1]))]),
        doc_lengths: BTreeMap::from([(1, 1)]),
        term_frequencies: BTreeMap::from([("budget".to_owned(), BTreeMap::from([(1, 1)]))]),
    }
    .write(&lexical)
    .unwrap();
    VectorIndex {
        vectors: BTreeMap::from([(1, vec![1, 2])]),
    }
    .write(&vector)
    .unwrap();
    HnswGraphIndex {
        links: BTreeMap::from([(1, BTreeSet::from([2]))]),
        dimension: 0,
        metric: 0,
    }
    .write(&hnsw)
    .unwrap();
    StorageManifest::default().store(&manifest).unwrap();

    for (kind, path) in [
        (StorageFormatKind::AclogWal, wal),
        (StorageFormatKind::Segment, segment),
        (StorageFormatKind::BitmapIndex, bitmap),
        (StorageFormatKind::LexicalIndex, lexical),
        (StorageFormatKind::VectorIndex, vector),
        (StorageFormatKind::HnswGraph, hnsw),
        (StorageFormatKind::Manifest, manifest),
    ] {
        let spec = storage_format_specs()
            .into_iter()
            .find(|spec| spec.kind == kind)
            .unwrap();
        let bytes = std::fs::read(path).unwrap();
        assert_eq!(&bytes[..spec.current_magic.len()], spec.current_magic);
    }
}

fn cell() -> SegmentCell {
    SegmentCell {
        candidate_id: 1,
        cell_id: 1,
        created_seq: 1,
        deleted_seq: None,
        payload: b"payload".to_vec(),
    }
}
