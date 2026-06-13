use cortex_core::CellId;
use cortex_engine::{CompactionPolicy, Database, DatabaseOptions};
use cortex_storage::manifest::ManifestCount;

fn opts_with_low_threshold() -> DatabaseOptions {
    DatabaseOptions {
        compaction_policy: CompactionPolicy {
            trigger_segment_count: 3,
            max_input_segments: 2,
            ..CompactionPolicy::default()
        },
        ..DatabaseOptions::default()
    }
}

fn put_many(db: &mut Database, start: u64, count: usize, payload: &str) {
    for i in 0..count {
        db.put_cell(
            CellId(start + i as u64),
            format!("{payload}-{i}").into_bytes(),
        )
        .unwrap();
    }
}

#[test]
fn incremental_compact_reduces_segment_count_and_preserves_data() {
    let dir = tempfile::tempdir().unwrap();
    {
        let mut db = Database::open_with_options(dir.path(), opts_with_low_threshold()).unwrap();
        put_many(&mut db, 1, 10, "batch-one");
        db.checkpoint().unwrap();
        put_many(&mut db, 11, 10, "batch-two");
        db.checkpoint().unwrap();
        put_many(&mut db, 21, 10, "batch-three");
        db.checkpoint().unwrap();

        assert_eq!(db.storage_stats().unwrap().live_segments, 3);

        let stats = db.incremental_compact().unwrap();
        assert_eq!(stats.segments_before, 3);
        assert_eq!(stats.segments_after, 2);
        assert!(stats.cells_compacted > 0);
        assert!(stats.duration_ms < 10_000);
        assert_eq!(db.manifest().segment_stats.len(), 2);
        let compacted_segment_id = db
            .manifest()
            .live_segments
            .iter()
            .map(|segment| segment.id)
            .max()
            .unwrap();
        let compacted_stats = db
            .manifest()
            .stats_for_segment(compacted_segment_id)
            .unwrap();
        assert_eq!(compacted_stats.row_count, 20);

        // Data from all batches is still visible.
        for i in 1..=30 {
            let bytes = db.get_latest_cell(CellId(i)).unwrap();
            let payload = String::from_utf8_lossy(&bytes);
            assert!(
                payload.contains("batch-one")
                    || payload.contains("batch-two")
                    || payload.contains("batch-three"),
                "cell {i} should retain data after compaction"
            );
        }
    }

    let db = Database::open_with_options(dir.path(), opts_with_low_threshold()).unwrap();
    assert_eq!(db.storage_stats().unwrap().live_segments, 2);
    for i in 1..=30 {
        assert!(
            db.get_latest_cell(CellId(i)).is_some(),
            "cell {i} should survive reopen after compaction"
        );
    }
    db.validate_storage().unwrap();
}

#[test]
fn incremental_compact_replaces_selected_segment_stats() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open_with_options(dir.path(), opts_with_low_threshold()).unwrap();
    db.put_cell(
        CellId(1),
        b"scope=project:a\nstatus=ready\ntype=fact\n\nalpha budget".to_vec(),
    )
    .unwrap();
    db.checkpoint().unwrap();
    db.put_cell(
        CellId(2),
        b"scope=project:b\nstatus=ready\ntype=fact\n\nbeta budget".to_vec(),
    )
    .unwrap();
    db.checkpoint().unwrap();
    db.put_cell(
        CellId(3),
        b"scope=project:c\nstatus=draft\ntype=document_block\n\ngamma".to_vec(),
    )
    .unwrap();
    db.checkpoint().unwrap();

    assert_eq!(db.manifest().segment_stats.len(), 3);
    db.incremental_compact().unwrap();

    assert_eq!(db.manifest().live_segments.len(), 2);
    assert_eq!(db.manifest().segment_stats.len(), 2);
    let compacted_segment_id = db
        .manifest()
        .live_segments
        .iter()
        .map(|segment| segment.id)
        .max()
        .unwrap();
    let compacted_stats = db
        .manifest()
        .stats_for_segment(compacted_segment_id)
        .unwrap();
    assert_eq!(compacted_stats.row_count, 2);
    assert_eq!(
        count_for(&compacted_stats.scope_counts, "project:a"),
        Some(1)
    );
    assert_eq!(
        count_for(&compacted_stats.scope_counts, "project:b"),
        Some(1)
    );
    assert_eq!(count_for(&compacted_stats.scope_counts, "project:c"), None);
    assert_eq!(compacted_stats.top_terms[0].term, "budget");
    assert_eq!(compacted_stats.top_terms[0].document_frequency, 2);
}

#[test]
fn incremental_compact_honors_memtable_updates_and_tombstones() {
    let dir = tempfile::tempdir().unwrap();
    {
        let mut db = Database::open_with_options(dir.path(), opts_with_low_threshold()).unwrap();
        db.put_cell(CellId(1), b"v1".to_vec()).unwrap();
        db.checkpoint().unwrap();
        db.put_cell(CellId(1), b"v2".to_vec()).unwrap();
        db.checkpoint().unwrap();
        put_many(&mut db, 2, 20, "filler");
        db.checkpoint().unwrap();

        let stats = db.incremental_compact().unwrap();
        assert!(stats.segments_before >= 3);
        assert!(stats.segments_after < stats.segments_before);
        assert_eq!(db.get_latest_cell(CellId(1)).unwrap(), b"v2");

        db.tombstone_cell(CellId(1)).unwrap();
        // Force another compaction after tombstone; the cell should not reappear.
        put_many(&mut db, 100, 5, "more");
        db.checkpoint().unwrap();
        let stats2 = db.incremental_compact().unwrap();
        assert!(stats2.segments_after < stats2.segments_before);
        assert!(db.get_latest_cell(CellId(1)).is_none());
    }

    let db = Database::open_with_options(dir.path(), opts_with_low_threshold()).unwrap();
    assert!(db.get_latest_cell(CellId(1)).is_none());
}

#[test]
fn maybe_incremental_compact_triggers_automatically() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open_with_options(dir.path(), opts_with_low_threshold()).unwrap();
    put_many(&mut db, 1, 5, "first");
    db.checkpoint().unwrap();
    put_many(&mut db, 6, 5, "second");
    db.checkpoint().unwrap();

    // Only two segments: below trigger.
    assert!(db.maybe_incremental_compact().unwrap().is_none());

    put_many(&mut db, 11, 5, "third");
    db.checkpoint().unwrap();

    let stats = db
        .maybe_incremental_compact()
        .unwrap()
        .expect("should compact");
    assert!(stats.segments_after < stats.segments_before);
}

fn count_for(counts: &[ManifestCount], key: &str) -> Option<u64> {
    counts
        .iter()
        .find(|count| count.key == key)
        .map(|count| count.count)
}
