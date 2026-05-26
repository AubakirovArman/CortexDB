use cortex_core::manifest::{Manifest, SegmentHandle, SegmentId};
use cortex_core::memtable::{MemTable, ReadTxn};
use cortex_core::{CellId, CommitSeq};

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

#[test]
fn memtable_gc_versions_before() {
    let mut table = MemTable::default();

    table.put_cell(CellId(1), CommitSeq(10), b"v1".to_vec());
    table
        .patch_cell(CellId(1), CommitSeq(20), b"v2".to_vec())
        .unwrap();

    table.put_cell(CellId(2), CommitSeq(12), b"v2_1".to_vec());
    table.tombstone_cell(CellId(2), CommitSeq(18)).unwrap();

    let stats_before = table.stats();
    assert_eq!(stats_before.version_count, 3);
    assert_eq!(stats_before.cell_count, 2);

    table.gc_versions_before(CommitSeq(15));
    let stats_15 = table.stats();
    assert_eq!(stats_15.version_count, 3);

    table.gc_versions_before(CommitSeq(25));
    let stats_25 = table.stats();
    assert_eq!(stats_25.cell_count, 1);
    assert_eq!(stats_25.version_count, 1);
    assert_eq!(
        table
            .read(
                ReadTxn {
                    read_seq: CommitSeq(30)
                },
                CellId(1)
            )
            .unwrap()
            .payload,
        b"v2"
    );
    assert!(table
        .read(
            ReadTxn {
                read_seq: CommitSeq(30)
            },
            CellId(2)
        )
        .is_none());
}
