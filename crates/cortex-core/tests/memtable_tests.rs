use cortex_core::manifest::{Manifest, SegmentHandle, SegmentId};
use cortex_core::memtable::{CellAccumulator, IndexDebt, MemTable, ReadTxn, SectionFragment};
use cortex_core::{CellId, CommitSeq};

#[test]
fn snapshot_reads_visible_version() {
    let mut table = MemTable::default();
    table.put_cell(CellId(1), CommitSeq(10), b"v1".to_vec());
    let version = table
        .read(
            ReadTxn {
                read_seq: CommitSeq(10),
            },
            CellId(1),
        )
        .unwrap();
    assert_eq!(version.payload, b"v1");
}

#[test]
fn old_read_txn_does_not_see_new_patch() {
    let mut table = MemTable::default();
    table.put_cell(CellId(1), CommitSeq(10), b"v1".to_vec());
    table
        .patch_cell(CellId(1), CommitSeq(20), b"v2".to_vec())
        .unwrap();
    let old = table
        .read(
            ReadTxn {
                read_seq: CommitSeq(15),
            },
            CellId(1),
        )
        .unwrap();
    let new = table
        .read(
            ReadTxn {
                read_seq: CommitSeq(20),
            },
            CellId(1),
        )
        .unwrap();
    assert_eq!(old.payload, b"v1");
    assert_eq!(new.payload, b"v2");
}

#[test]
fn tombstone_hides_cell_after_delete_seq() {
    let mut table = MemTable::default();
    table.put_cell(CellId(1), CommitSeq(10), b"v1".to_vec());
    table.tombstone_cell(CellId(1), CommitSeq(20)).unwrap();
    assert!(table
        .read(
            ReadTxn {
                read_seq: CommitSeq(19)
            },
            CellId(1)
        )
        .is_some());
    assert!(table
        .read(
            ReadTxn {
                read_seq: CommitSeq(20)
            },
            CellId(1)
        )
        .is_none());
}

#[test]
fn multiple_versions_resolve_latest_visible() {
    let mut table = MemTable::default();
    table.put_cell(CellId(1), CommitSeq(10), b"v1".to_vec());
    table
        .patch_cell(CellId(1), CommitSeq(20), b"v2".to_vec())
        .unwrap();
    table
        .patch_cell(CellId(1), CommitSeq(30), b"v3".to_vec())
        .unwrap();
    let version = table
        .read(
            ReadTxn {
                read_seq: CommitSeq(25),
            },
            CellId(1),
        )
        .unwrap();
    assert_eq!(version.payload, b"v2");
}

#[test]
fn accumulator_merges_sections_deterministically() {
    let mut accumulator = CellAccumulator::default();
    accumulator.push(SectionFragment {
        tag: 2,
        data: b"b".to_vec(),
    });
    accumulator.push(SectionFragment {
        tag: 1,
        data: b"a".to_vec(),
    });
    let merged = accumulator.finish();
    assert_eq!(merged[0].tag, 1);
    assert_eq!(merged[1].tag, 2);
}

#[test]
fn delta_depth_and_compaction_priority_increase_with_patches() {
    let mut table = MemTable::default();
    table.put_cell(CellId(1), CommitSeq(10), b"v1".to_vec());
    table
        .patch_cell(CellId(1), CommitSeq(20), b"v2".to_vec())
        .unwrap();
    table
        .patch_cell(CellId(1), CommitSeq(30), b"v3".to_vec())
        .unwrap();
    let version = table
        .read(
            ReadTxn {
                read_seq: CommitSeq(30),
            },
            CellId(1),
        )
        .unwrap();
    assert_eq!(version.delta_depth, 2);
    assert_eq!(table.compaction_priority(CellId(1)), Some(2));
}

#[test]
fn index_debt_counts_all_index_work() {
    let debt = IndexDebt {
        bitmap: 1,
        lexical: 2,
        vector: 3,
    };
    assert_eq!(debt.total(), 6);
}

#[test]
fn manifest_tracks_live_and_retired_segments() {
    let mut manifest = Manifest::default();
    manifest.add_live_segment(SegmentHandle {
        id: SegmentId(1),
        generation: 1,
    });
    manifest.retire_segment(SegmentId(1));
    assert_eq!(manifest.generation, 2);
    assert!(manifest.live_segments.is_empty());
    assert_eq!(manifest.retired_segments[0].id, SegmentId(1));
}
