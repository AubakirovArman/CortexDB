use std::collections::{BTreeMap, BTreeSet};

use cortex_storage::indexes::{BitmapIndex, LexicalIndex};
use cortex_storage::manifest::{ManifestSegment, StorageManifest};
use cortex_storage::segment::{SegmentCell, SegmentReader, SegmentWriter};
use cortex_storage::StorageError;

#[test]
fn acs_segment_roundtrips_cells() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("0001.acs");
    let cells = vec![
        SegmentCell {
            cell_id: 1,
            created_seq: 7,
            deleted_seq: None,
            payload: b"one".to_vec(),
        },
        SegmentCell {
            cell_id: 2,
            created_seq: 8,
            deleted_seq: Some(9),
            payload: b"two".to_vec(),
        },
    ];
    SegmentWriter::write(&path, &cells).unwrap();
    assert_eq!(SegmentReader::read(&path).unwrap(), cells);
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
}

#[test]
fn aci_lexical_index_roundtrips_terms() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("0001.aci");
    let index = LexicalIndex {
        terms: BTreeMap::from([
            ("budget".to_owned(), BTreeSet::from([1, 2])),
            ("ready".to_owned(), BTreeSet::from([2])),
        ]),
    };
    index.write(&path).unwrap();
    assert_eq!(LexicalIndex::read(&path).unwrap(), index);
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
    };

    manifest.store(&path).unwrap();

    assert_eq!(StorageManifest::load(&path).unwrap(), manifest);
    assert!(!dir.path().join("manifest.acm.tmp").exists());
}

#[test]
fn invalid_storage_files_are_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let segment = dir.path().join("bad.acs");
    let bitmap = dir.path().join("bad.acb");
    let lexical = dir.path().join("bad.aci");
    let manifest = dir.path().join("bad.acm");
    std::fs::write(&segment, b"bad").unwrap();
    std::fs::write(&bitmap, b"bad").unwrap();
    std::fs::write(&lexical, b"bad").unwrap();
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
        StorageManifest::load(&manifest).unwrap_err(),
        StorageError::InvalidManifestFile
    ));
}
