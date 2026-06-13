use cortex_core::{CellId, CommitSeq};
use cortex_engine::{Database, DatabaseOptions, EngineFeatureFlags, PayloadResidency};

#[test]
fn lazy_restart_matrix_survives_checkpoint_and_compact_wal_tails() {
    type RestartCase = (
        &'static str,
        fn(&mut Database),
        CommitSeq,
        Option<&'static [u8]>,
    );

    let cases: &[RestartCase] = &[
        (
            "checkpoint_patch_tail",
            checkpoint_patch_tail,
            CommitSeq(2),
            Some(b"patched-tail"),
        ),
        (
            "checkpoint_tombstone_tail",
            checkpoint_tombstone_tail,
            CommitSeq(2),
            None,
        ),
        (
            "compact_patch_tail",
            compact_patch_tail,
            CommitSeq(2),
            Some(b"patched-after-compact"),
        ),
        (
            "compact_tombstone_tail",
            compact_tombstone_tail,
            CommitSeq(2),
            None,
        ),
    ];

    for (name, setup, expected_seq, expected_payload) in cases {
        let dir = tempfile::tempdir().unwrap();
        {
            let mut db = Database::open(dir.path()).unwrap();
            setup(&mut db);
        }

        let db = Database::open_with_options(dir.path(), lazy_options()).unwrap();
        assert_eq!(db.current_seq(), *expected_seq, "{name}");
        assert_eq!(
            db.get_latest_cell(CellId(1)).as_deref(),
            *expected_payload,
            "{name}"
        );
        db.validate_storage().unwrap();
    }
}

#[test]
fn lazy_corruption_matrix_fails_closed_or_reports_corruption() {
    for (path, expected) in [
        ("segments/segment-1.acs", "segment"),
        ("manifest.acm", "manifest"),
        ("segments/segment-1.acb", "bitmap index 1"),
        ("segments/segment-1.aci", "lexical index 1"),
        ("segments/segment-1.acv", "vector index 1"),
    ] {
        let dir = tempfile::tempdir().unwrap();
        write_checkpoint(dir.path());
        corrupt_last_byte(&dir.path().join(path));
        assert_lazy_fails_closed_or_reports(dir.path(), expected);
    }

    let dir = tempfile::tempdir().unwrap();
    write_hnsw_checkpoint(dir.path());
    corrupt_last_byte(&dir.path().join("segments").join("segment-1.ach"));
    assert_lazy_fails_closed_or_reports(dir.path(), "hnsw graph 1");
}

fn checkpoint_patch_tail(db: &mut Database) {
    db.put_cell(CellId(1), b"base".to_vec()).unwrap();
    db.checkpoint().unwrap();
    db.patch_cell(CellId(1), b"patched-tail".to_vec()).unwrap();
}

fn checkpoint_tombstone_tail(db: &mut Database) {
    db.put_cell(CellId(1), b"base".to_vec()).unwrap();
    db.checkpoint().unwrap();
    db.tombstone_cell(CellId(1)).unwrap();
}

fn compact_patch_tail(db: &mut Database) {
    db.put_cell(CellId(1), b"base".to_vec()).unwrap();
    db.compact().unwrap();
    db.patch_cell(CellId(1), b"patched-after-compact".to_vec())
        .unwrap();
}

fn compact_tombstone_tail(db: &mut Database) {
    db.put_cell(CellId(1), b"base".to_vec()).unwrap();
    db.compact().unwrap();
    db.tombstone_cell(CellId(1)).unwrap();
}

fn write_checkpoint(root: &std::path::Path) {
    let mut db = Database::open(root).unwrap();
    db.put_cell(CellId(1), b"scope=default\nstatus=ready\none".to_vec())
        .unwrap();
    db.checkpoint().unwrap();
}

fn write_hnsw_checkpoint(root: &std::path::Path) {
    let mut db = Database::open_with_options(
        root,
        DatabaseOptions {
            feature_flags: EngineFeatureFlags::production_safe().with_experimental_hnsw(true),
            ..DatabaseOptions::default()
        },
    )
    .unwrap();
    db.put_cell(
        CellId(1),
        b"scope=default\nstatus=ready\nvector=1,0\n\none".to_vec(),
    )
    .unwrap();
    db.checkpoint().unwrap();
}

fn assert_lazy_fails_closed_or_reports(root: &std::path::Path, expected: &str) {
    if let Ok(db) = Database::open_with_options(root, lazy_options()) {
        let report = db.validate_storage_report();
        assert!(
            report.errors.iter().any(|error| error.contains(expected)),
            "expected lazy validation error containing {expected:?}, got {:?}",
            report.errors
        );
    }
}

fn lazy_options() -> DatabaseOptions {
    DatabaseOptions {
        payload_residency: PayloadResidency::Lazy,
        ..DatabaseOptions::default()
    }
}

fn corrupt_last_byte(path: &std::path::Path) {
    let mut bytes = std::fs::read(path).unwrap();
    *bytes.last_mut().unwrap() ^= 0xff;
    std::fs::write(path, bytes).unwrap();
}
