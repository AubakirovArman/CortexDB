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
fn read_at_uses_historical_visibility() {
    let mut table = MemTable::default();
    table.put_cell(CellId(1), CommitSeq(10), b"v1".to_vec());
    table
        .patch_cell(CellId(1), CommitSeq(20), b"v2".to_vec())
        .unwrap();

    assert_eq!(
        table.read_at(CommitSeq(15), CellId(1)).unwrap().payload,
        b"v1"
    );
    assert_eq!(
        table.read_at(CommitSeq(20), CellId(1)).unwrap().payload,
        b"v2"
    );
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
fn tombstone_marker_without_base_is_never_visible() {
    let mut table = MemTable::default();
    table.record_tombstone(CellId(9), CommitSeq(20));
    assert!(table
        .read(
            ReadTxn {
                read_seq: CommitSeq(19)
            },
            CellId(9)
        )
        .is_none());
    assert!(table
        .read(
            ReadTxn {
                read_seq: CommitSeq(20)
            },
            CellId(9)
        )
        .is_none());
    assert_eq!(
        table.tombstones_after(CommitSeq(10)),
        vec![(CellId(9), CommitSeq(20))]
    );
}

#[test]
fn memtable_stats_track_versions_and_tombstones() {
    let mut table = MemTable::default();
    table.put_cell(CellId(1), CommitSeq(10), b"v1".to_vec());
    table
        .patch_cell(CellId(1), CommitSeq(20), b"v2".to_vec())
        .unwrap();
    table.record_tombstone(CellId(2), CommitSeq(30));

    let stats = table.stats();
    assert_eq!(stats.cell_count, 2);
    assert_eq!(stats.live_cell_count, 1);
    assert_eq!(stats.deleted_cell_count, 1);
    assert_eq!(stats.version_count, 3);
    assert_eq!(stats.tombstone_count, 1);
    assert_eq!(stats.max_delta_depth, 1);
    assert_eq!(stats.payload_bytes, 4);
    assert_eq!(table.latest_seq(), CommitSeq(30));
    assert!(table.contains_visible(
        ReadTxn {
            read_seq: CommitSeq(20)
        },
        CellId(1)
    ));
}

#[test]
fn live_deleted_iterators_are_deterministic() {
    let mut table = MemTable::default();
    table.put_cell(CellId(3), CommitSeq(10), b"c".to_vec());
    table.put_cell(CellId(1), CommitSeq(11), b"a".to_vec());
    table.put_cell(CellId(2), CommitSeq(12), b"b".to_vec());
    table.tombstone_cell(CellId(2), CommitSeq(20)).unwrap();

    assert_eq!(
        table.live_cell_ids(ReadTxn::at(CommitSeq(20))),
        vec![CellId(1), CellId(3)]
    );
    assert_eq!(table.deleted_cell_ids(), vec![CellId(2)]);
}

#[test]
fn range_scan_obeys_mvcc_and_cell_order() {
    let mut table = MemTable::default();
    table.put_cell(CellId(1), CommitSeq(10), b"a".to_vec());
    table.put_cell(CellId(2), CommitSeq(11), b"b1".to_vec());
    table.put_cell(CellId(3), CommitSeq(12), b"c".to_vec());
    table
        .patch_cell(CellId(2), CommitSeq(20), b"b2".to_vec())
        .unwrap();

    let old = table.range_scan(ReadTxn::at(CommitSeq(15)), CellId(1), CellId(3));
    assert_eq!(
        old.iter()
            .map(|version| version.payload.as_slice())
            .collect::<Vec<_>>(),
        vec![b"a".as_slice(), b"b1".as_slice(), b"c".as_slice()]
    );

    let new = table.range_scan(ReadTxn::at(CommitSeq(20)), CellId(2), CellId(3));
    assert_eq!(
        new.iter()
            .map(|version| version.payload.as_slice())
            .collect::<Vec<_>>(),
        vec![b"b2".as_slice(), b"c".as_slice()]
    );
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
