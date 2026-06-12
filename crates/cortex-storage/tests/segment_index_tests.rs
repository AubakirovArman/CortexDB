use std::collections::{BTreeMap, BTreeSet};

use cortex_storage::indexes::{BitmapIndex, LexicalIndex};
use cortex_storage::manifest::{ManifestSegment, StorageManifest};
use cortex_storage::segment::{SegmentCell, SegmentCellRef, SegmentReader, SegmentWriter};
use cortex_storage::vectors::VectorIndex;
use cortex_storage::StorageError;

#[test]
fn acs_segment_roundtrips_cells() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("0001.acs");
    let cells = vec![
        SegmentCell {
            candidate_id: 1,
            cell_id: 1,
            created_seq: 7,
            deleted_seq: None,
            payload: b"one".to_vec(),
        },
        SegmentCell {
            candidate_id: 2,
            cell_id: 2,
            created_seq: 8,
            deleted_seq: Some(9),
            payload: b"two".to_vec(),
        },
    ];
    SegmentWriter::write(&path, &cells).unwrap();
    assert_eq!(SegmentReader::read(&path).unwrap(), cells);
    assert!(!dir.path().join("0001.acs.tmp").exists());
}

#[test]
fn acs_segment_persists_cells_in_candidate_order() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("ordered.acs");
    SegmentWriter::write(
        &path,
        &[
            segment_cell(3, 30),
            segment_cell(1, 10),
            segment_cell(2, 20),
        ],
    )
    .unwrap();

    let candidates = SegmentReader::read(&path)
        .unwrap()
        .into_iter()
        .map(|cell| cell.candidate_id)
        .collect::<Vec<_>>();
    assert_eq!(candidates, vec![1, 2, 3]);
}

#[test]
fn acs_segment_writes_borrowed_cells_without_owned_payloads() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("borrowed.acs");
    let first = b"one".to_vec();
    let second = b"two".to_vec();
    let cells = [
        SegmentCellRef {
            candidate_id: 2,
            cell_id: 20,
            created_seq: 8,
            deleted_seq: None,
            descriptor: None,
            payload: &second,
        },
        SegmentCellRef {
            candidate_id: 1,
            cell_id: 10,
            created_seq: 7,
            deleted_seq: None,
            descriptor: None,
            payload: &first,
        },
    ];

    SegmentWriter::write_refs(&path, &cells).unwrap();

    assert_eq!(
        SegmentReader::read(&path).unwrap(),
        vec![
            SegmentCell {
                candidate_id: 1,
                cell_id: 10,
                created_seq: 7,
                deleted_seq: None,
                payload: b"one".to_vec(),
            },
            SegmentCell {
                candidate_id: 2,
                cell_id: 20,
                created_seq: 8,
                deleted_seq: None,
                payload: b"two".to_vec(),
            },
        ]
    );
}

#[test]
fn segment_lookup_finds_cells_by_candidate_and_cell_id() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("lookup.acs");
    SegmentWriter::write(&path, &[segment_cell(2, 20), segment_cell(1, 10)]).unwrap();

    let lookup = SegmentReader::read_lookup(&path).unwrap();

    assert_eq!(lookup.cells().len(), 2);
    assert_eq!(lookup.cell_by_candidate(1).unwrap().cell_id, 10);
    assert_eq!(lookup.cell_by_candidate(2).unwrap().cell_id, 20);
    assert_eq!(lookup.cell_by_cell_id(20).unwrap().candidate_id, 2);
    assert!(lookup.cell_by_candidate(99).is_none());
}

#[test]
fn acb_bitmap_index_roundtrips_sorted_sets() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("0001.acb");
    let index = BitmapIndex {
        bitmaps: BTreeMap::from([
            (10, BTreeSet::from([1, 3, 5])),
            (20, BTreeSet::from([2, 4])),
        ]),
    };
    index.write(&path).unwrap();
    assert_eq!(BitmapIndex::read(&path).unwrap(), index);
    assert!(!dir.path().join("0001.acb.tmp").exists());
}

#[test]
fn acv_vector_index_roundtrips_vectors() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("0001.acv");
    let index = VectorIndex {
        vectors: BTreeMap::from([(1, vec![3, -1, 9]), (2, vec![0, 4])]),
    };
    index.write(&path).unwrap();
    assert_eq!(VectorIndex::read(&path).unwrap(), index);
    assert!(!dir.path().join("0001.acv.tmp").exists());
}

#[test]
fn manifest_roundtrips_checkpoint_segments() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("manifest.acm");
    let mut manifest = StorageManifest::default();
    manifest.checkpoint_segment(ManifestSegment {
        id: 1,
        generation: 1,
        checkpoint_seq: 9,
        cell_count: 2,
    });
    manifest.store(&path).unwrap();
    assert_eq!(StorageManifest::load(&path).unwrap(), manifest);
}

#[test]
fn manifest_store_is_atomic_and_ignores_leftover_tmp() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("manifest.acm");
    std::fs::write(dir.path().join("manifest.acm.tmp"), b"bad").unwrap();
    let manifest = StorageManifest {
        generation: 3,
        checkpoint_seq: 11,
        live_segments: vec![cortex_storage::manifest::ManifestSegment {
            id: 2,
            generation: 3,
            checkpoint_seq: 11,
            cell_count: 4,
        }],
        retired_segments: Vec::new(),
        hnsw_profile: None,
        vector_profile: None,
        hnsw_no_fallback_profile: None,
    };

    manifest.store(&path).unwrap();

    assert_eq!(StorageManifest::load(&path).unwrap(), manifest);
    assert!(!dir.path().join("manifest.acm.tmp").exists());
}

#[test]
fn manifest_forward_compatibility() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("manifest.acm");
    let mut manifest = StorageManifest::default();
    manifest.checkpoint_segment(ManifestSegment {
        id: 1,
        generation: 1,
        checkpoint_seq: 9,
        cell_count: 2,
    });
    manifest.store(&path).unwrap();

    let mut bytes = std::fs::read(&path).unwrap();
    bytes.truncate(bytes.len() - 4);
    bytes.extend_from_slice(&[42, 43, 44, 45]);
    let checksum = crc32c::crc32c(&bytes);
    bytes.extend_from_slice(&checksum.to_le_bytes());
    std::fs::write(&path, &bytes).unwrap();

    let loaded = StorageManifest::load(&path).unwrap();
    assert_eq!(loaded, manifest);
}

#[test]
fn invalid_storage_files_are_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let segment = dir.path().join("bad.acs");
    let bitmap = dir.path().join("bad.acb");
    let lexical = dir.path().join("bad.aci");
    let vector = dir.path().join("bad.acv");
    let manifest = dir.path().join("bad.acm");
    std::fs::write(&segment, b"bad").unwrap();
    std::fs::write(&bitmap, b"bad").unwrap();
    std::fs::write(&lexical, b"bad").unwrap();
    std::fs::write(&vector, b"bad").unwrap();
    std::fs::write(&manifest, b"bad").unwrap();
    assert!(matches!(
        SegmentReader::read(&segment).unwrap_err(),
        StorageError::InvalidSegmentFile
    ));
    assert!(matches!(
        BitmapIndex::read(&bitmap).unwrap_err(),
        StorageError::InvalidBitmapIndexFile
    ));
    assert!(matches!(
        LexicalIndex::read(&lexical).unwrap_err(),
        StorageError::InvalidLexicalIndexFile
    ));
    assert!(matches!(
        VectorIndex::read(&vector).unwrap_err(),
        StorageError::InvalidVectorIndexFile
    ));
    assert!(matches!(
        StorageManifest::load(&manifest).unwrap_err(),
        StorageError::InvalidManifestFile
    ));
}

#[test]
fn checksum_corruption_is_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let segment = dir.path().join("ok.acs");
    let bitmap = dir.path().join("ok.acb");
    let lexical = dir.path().join("ok.aci");
    let vector = dir.path().join("ok.acv");
    let manifest = dir.path().join("manifest.acm");

    SegmentWriter::write(
        &segment,
        &[SegmentCell {
            candidate_id: 1,
            cell_id: 1,
            created_seq: 1,
            deleted_seq: None,
            payload: b"one".to_vec(),
        }],
    )
    .unwrap();
    BitmapIndex {
        bitmaps: BTreeMap::from([(1, BTreeSet::from([1]))]),
    }
    .write(&bitmap)
    .unwrap();
    LexicalIndex {
        terms: BTreeMap::from([("one".to_owned(), BTreeSet::from([1]))]),
        doc_lengths: BTreeMap::from([(1, 1)]),
        term_frequencies: BTreeMap::from([("one".to_owned(), BTreeMap::from([(1, 1)]))]),
        ..LexicalIndex::default()
    }
    .write(&lexical)
    .unwrap();
    VectorIndex {
        vectors: BTreeMap::from([(1, vec![1, 2])]),
    }
    .write(&vector)
    .unwrap();
    StorageManifest::default().store(&manifest).unwrap();

    corrupt_last_byte(&segment);
    corrupt_last_byte(&bitmap);
    corrupt_last_byte(&lexical);
    corrupt_last_byte(&vector);
    corrupt_last_byte(&manifest);

    assert!(matches!(
        SegmentReader::read(&segment).unwrap_err(),
        StorageError::InvalidSegmentFile
    ));
    assert!(matches!(
        BitmapIndex::read(&bitmap).unwrap_err(),
        StorageError::InvalidBitmapIndexFile
    ));
    assert!(matches!(
        LexicalIndex::read(&lexical).unwrap_err(),
        StorageError::InvalidLexicalIndexFile
    ));
    assert!(matches!(
        VectorIndex::read(&vector).unwrap_err(),
        StorageError::InvalidVectorIndexFile
    ));
    assert!(matches!(
        StorageManifest::load(&manifest).unwrap_err(),
        StorageError::InvalidManifestFile
    ));
}

fn corrupt_last_byte(path: &std::path::Path) {
    let mut bytes = std::fs::read(path).unwrap();
    let last = bytes.last_mut().unwrap();
    *last ^= 0xff;
    std::fs::write(path, bytes).unwrap();
}

fn segment_cell(candidate_id: u32, cell_id: u64) -> SegmentCell {
    SegmentCell {
        candidate_id,
        cell_id,
        created_seq: candidate_id as u64,
        deleted_seq: None,
        payload: cell_id.to_le_bytes().to_vec(),
    }
}
