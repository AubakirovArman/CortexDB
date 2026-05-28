use std::collections::{BTreeMap, BTreeSet};

use cortex_core::CellId;
use cortex_engine::Database;
use cortex_storage::hnsw::HnswGraphIndex;
use cortex_storage::indexes::{BitmapIndex, LexicalIndex};
use cortex_storage::manifest::{ManifestSegment, StorageManifest};
use cortex_storage::segment::{SegmentCell, SegmentWriter};
use cortex_storage::vectors::VectorIndex;

#[test]
fn duplicate_live_segment_id_fails_validation() {
    let dir = tempfile::tempdir().unwrap();
    write_bundle(dir.path(), 1, 1, 1, CellId(1));
    write_manifest(
        dir.path(),
        1,
        vec![manifest_segment(1, 1, 1), manifest_segment(1, 1, 1)],
        Vec::new(),
    );

    let db = Database::open(dir.path()).unwrap();
    let error = db.validate_storage().unwrap_err().to_string();
    assert!(error.contains("duplicate live segment id"));
}

#[test]
fn duplicate_candidate_id_across_segments_fails_validation() {
    let dir = tempfile::tempdir().unwrap();
    write_bundle(dir.path(), 1, 1, 1, CellId(1));
    write_bundle(dir.path(), 2, 2, 1, CellId(2));
    write_manifest(
        dir.path(),
        2,
        vec![manifest_segment(1, 1, 1), manifest_segment(2, 2, 1)],
        Vec::new(),
    );

    let db = Database::open(dir.path()).unwrap();
    let error = db.validate_storage().unwrap_err().to_string();
    assert!(error.contains("maps to multiple cells"));
}

#[test]
fn manifest_checkpoint_seq_regression_fails_validation() {
    let dir = tempfile::tempdir().unwrap();
    write_bundle(dir.path(), 1, 5, 1, CellId(1));
    write_manifest(dir.path(), 4, vec![manifest_segment(1, 5, 1)], Vec::new());

    let db = Database::open(dir.path()).unwrap();
    let error = db.validate_storage().unwrap_err().to_string();
    assert!(error.contains("is behind segment"));
}

#[test]
fn segment_with_zero_candidate_id_is_invalid() {
    let dir = tempfile::tempdir().unwrap();
    write_bundle(dir.path(), 1, 1, 0, CellId(1));
    write_manifest(dir.path(), 1, vec![manifest_segment(1, 1, 1)], Vec::new());

    let db = Database::open(dir.path()).unwrap();
    let error = db.validate_storage().unwrap_err().to_string();
    assert!(error.contains("invalid candidate id"));
}

#[test]
fn live_retired_segment_overlap_fails_validation() {
    let dir = tempfile::tempdir().unwrap();
    write_bundle(dir.path(), 1, 1, 1, CellId(1));
    write_manifest(
        dir.path(),
        1,
        vec![manifest_segment(1, 1, 1)],
        vec![manifest_segment(1, 1, 1)],
    );

    let db = Database::open(dir.path()).unwrap();
    let error = db.validate_storage().unwrap_err().to_string();
    assert!(error.contains("conflicts with manifest references"));
}

#[test]
fn hnsw_graph_candidate_without_vector_fails_validation() {
    let dir = tempfile::tempdir().unwrap();
    write_bundle(dir.path(), 1, 1, 1, CellId(1));
    let segments = dir.path().join("segments");
    VectorIndex {
        vectors: BTreeMap::from([(1, vec![10, 0])]),
    }
    .write(segments.join("segment-1.acv"))
    .unwrap();
    HnswGraphIndex {
        links: BTreeMap::from([(1, BTreeSet::from([2]))]),
        dimension: 2,
        metric: 0,
    }
    .write(segments.join("segment-1.ach"))
    .unwrap();
    write_manifest(dir.path(), 1, vec![manifest_segment(1, 1, 1)], Vec::new());

    let db = Database::open(dir.path()).unwrap();
    let error = db.validate_storage().unwrap_err().to_string();

    assert!(error.contains("hnsw graph 1 integrity"));
    assert!(error.contains("missing_neighbor_links=1"));
}

#[test]
fn vector_index_dimension_mismatch_fails_validation() {
    let dir = tempfile::tempdir().unwrap();
    write_bundle(dir.path(), 1, 1, 1, CellId(1));
    VectorIndex {
        vectors: BTreeMap::from([(1, vec![10, 0]), (2, vec![10])]),
    }
    .write(dir.path().join("segments").join("segment-1.acv"))
    .unwrap();
    write_manifest(dir.path(), 1, vec![manifest_segment(1, 1, 1)], Vec::new());

    let db = Database::open(dir.path()).unwrap();
    let error = db.validate_storage().unwrap_err().to_string();

    assert!(error.contains("vector index 1 dimensions"));
    assert!(error.contains("mismatched_vectors=1"));
}

#[test]
fn validation_report_collects_multiple_errors() {
    let dir = tempfile::tempdir().unwrap();
    write_bundle(dir.path(), 1, 5, 0, CellId(1));
    write_bundle(dir.path(), 2, 6, 0, CellId(2));
    write_manifest(
        dir.path(),
        4,
        vec![manifest_segment(1, 5, 1), manifest_segment(2, 6, 1)],
        vec![manifest_segment(1, 5, 1)],
    );

    let db = Database::open(dir.path()).unwrap();
    let report = db.validate_storage_report();
    assert!(!report.errors.is_empty());
    assert!(report
        .errors
        .iter()
        .any(|error| error.contains("behind segment")));
    assert!(report
        .errors
        .iter()
        .any(|error| error.contains("invalid candidate id")));
    assert!(report
        .errors
        .iter()
        .any(|error| error.contains("maps to multiple cells")));
    assert!(report
        .errors
        .iter()
        .any(|error| error.contains("conflicts with manifest references")));
}

fn write_bundle(root: &std::path::Path, segment_id: u64, seq: u64, candidate: u32, cell: CellId) {
    let segments = root.join("segments");
    std::fs::create_dir_all(&segments).unwrap();
    SegmentWriter::write(
        segments.join(format!("segment-{segment_id}.acs")),
        &[SegmentCell {
            candidate_id: candidate,
            cell_id: cell.0,
            created_seq: seq,
            deleted_seq: None,
            payload: b"scope=default\nstatus=ready\npayload".to_vec(),
        }],
    )
    .unwrap();
    BitmapIndex::default()
        .write(segments.join(format!("segment-{segment_id}.acb")))
        .unwrap();
    LexicalIndex::default()
        .write(segments.join(format!("segment-{segment_id}.aci")))
        .unwrap();
    VectorIndex::default()
        .write(segments.join(format!("segment-{segment_id}.acv")))
        .unwrap();
    HnswGraphIndex::default()
        .write(segments.join(format!("segment-{segment_id}.ach")))
        .unwrap();
}

fn write_manifest(
    root: &std::path::Path,
    checkpoint_seq: u64,
    live_segments: Vec<ManifestSegment>,
    retired_segments: Vec<ManifestSegment>,
) {
    StorageManifest {
        generation: 1,
        checkpoint_seq,
        live_segments,
        retired_segments,
    }
    .store(root.join("manifest.acm"))
    .unwrap();
}

fn manifest_segment(id: u64, checkpoint_seq: u64, cell_count: u32) -> ManifestSegment {
    ManifestSegment {
        id,
        generation: id,
        checkpoint_seq,
        cell_count,
    }
}
