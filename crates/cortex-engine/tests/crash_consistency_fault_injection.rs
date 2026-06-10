use std::fs;
use std::path::Path;

use cortex_core::{CellId, CommitSeq};
use cortex_engine::Database;
use cortex_storage::hnsw::HnswGraphIndex;
use cortex_storage::indexes::{BitmapIndex, LexicalIndex};
use cortex_storage::segment::{SegmentCell, SegmentWriter};
use cortex_storage::vectors::VectorIndex;

const SCENARIOS: usize = 1_000;

#[derive(Clone, Copy)]
enum FaultPhase {
    CleanCheckpoint,
    OrphanSegmentBeforeManifest,
    OrphanBundleBeforeManifest,
    CheckpointManifestBeforeWalTruncate,
    CheckpointTmpLeftovers,
    CompactManifestBeforeWalTruncate,
    PartialWalTail,
    PartialBundleBeforeManifest,
    CorruptOrphanBeforeManifest,
    CompactBundleBeforeManifest,
}

impl FaultPhase {
    fn from_index(index: usize) -> Self {
        match index % 10 {
            0 => Self::CleanCheckpoint,
            1 => Self::OrphanSegmentBeforeManifest,
            2 => Self::OrphanBundleBeforeManifest,
            3 => Self::CheckpointManifestBeforeWalTruncate,
            4 => Self::CheckpointTmpLeftovers,
            5 => Self::CompactManifestBeforeWalTruncate,
            6 => Self::PartialWalTail,
            7 => Self::PartialBundleBeforeManifest,
            8 => Self::CorruptOrphanBeforeManifest,
            _ => Self::CompactBundleBeforeManifest,
        }
    }

    fn index(self) -> usize {
        match self {
            Self::CleanCheckpoint => 0,
            Self::OrphanSegmentBeforeManifest => 1,
            Self::OrphanBundleBeforeManifest => 2,
            Self::CheckpointManifestBeforeWalTruncate => 3,
            Self::CheckpointTmpLeftovers => 4,
            Self::CompactManifestBeforeWalTruncate => 5,
            Self::PartialWalTail => 6,
            Self::PartialBundleBeforeManifest => 7,
            Self::CorruptOrphanBeforeManifest => 8,
            Self::CompactBundleBeforeManifest => 9,
        }
    }
}

#[test]
fn wal_checkpoint_fault_injection_preserves_last_committed_state() {
    let mut executed = [0usize; 10];
    for scenario in 0..SCENARIOS {
        let phase = FaultPhase::from_index(scenario);
        run_scenario(scenario as u64, phase);
        executed[phase.index()] += 1;
    }
    assert_eq!(executed.into_iter().sum::<usize>(), SCENARIOS);
    assert!(executed.into_iter().all(|count| count > 0));
}

#[test]
fn published_torn_checkpoint_files_fail_closed_or_validate_bad() {
    let dir = tempfile::tempdir().unwrap();
    {
        let mut db = Database::open(dir.path()).unwrap();
        db.put_cell(CellId(1), payload("published", 1)).unwrap();
        db.checkpoint().unwrap();
    }

    let segment = dir.path().join("segments").join("segment-1.acs");
    corrupt_last_byte(&segment);
    assert!(Database::open(dir.path()).is_err());
}

fn run_scenario(seed: u64, phase: FaultPhase) {
    match phase {
        FaultPhase::CleanCheckpoint => clean_checkpoint(seed),
        FaultPhase::OrphanSegmentBeforeManifest => orphan_segment_before_manifest(seed),
        FaultPhase::OrphanBundleBeforeManifest => orphan_bundle_before_manifest(seed),
        FaultPhase::CheckpointManifestBeforeWalTruncate => {
            checkpoint_manifest_before_wal_truncate(seed)
        }
        FaultPhase::CheckpointTmpLeftovers => checkpoint_tmp_leftovers(seed),
        FaultPhase::CompactManifestBeforeWalTruncate => compact_manifest_before_wal_truncate(seed),
        FaultPhase::PartialWalTail => partial_wal_tail(seed),
        FaultPhase::PartialBundleBeforeManifest => partial_bundle_before_manifest(seed),
        FaultPhase::CorruptOrphanBeforeManifest => corrupt_orphan_before_manifest(seed),
        FaultPhase::CompactBundleBeforeManifest => compact_bundle_before_manifest(seed),
    }
}

fn clean_checkpoint(seed: u64) {
    let dir = tempfile::tempdir().unwrap();
    {
        let mut db = Database::open(dir.path()).unwrap();
        db.put_cell(CellId(1), payload("clean-a", seed)).unwrap();
        db.put_cell(CellId(2), payload("clean-b", seed)).unwrap();
        db.checkpoint().unwrap();
    }
    assert_visible(
        dir.path(),
        CommitSeq(2),
        &[
            (CellId(1), payload("clean-a", seed)),
            (CellId(2), payload("clean-b", seed)),
        ],
        &[CellId(99)],
    );
}

fn orphan_segment_before_manifest(seed: u64) {
    let dir = base_checkpoint_with_wal_tail(seed, "orphan-segment");
    write_orphan_segment_file_only(dir.path(), 99, CellId(99), seed);
    assert_visible(
        dir.path(),
        CommitSeq(2),
        &[
            (CellId(1), payload("base", seed)),
            (CellId(2), payload("tail", seed)),
        ],
        &[CellId(99)],
    );
}

fn orphan_bundle_before_manifest(seed: u64) {
    let dir = base_checkpoint_with_wal_tail(seed, "orphan-bundle");
    write_orphan_bundle(dir.path(), 99, CellId(99), seed);
    assert_visible(
        dir.path(),
        CommitSeq(2),
        &[
            (CellId(1), payload("base", seed)),
            (CellId(2), payload("tail", seed)),
        ],
        &[CellId(99)],
    );
}

fn checkpoint_manifest_before_wal_truncate(seed: u64) {
    let dir = tempfile::tempdir().unwrap();
    let saved_wal = {
        let mut db = Database::open(dir.path()).unwrap();
        db.put_cell(CellId(1), payload("checkpoint-a", seed))
            .unwrap();
        db.put_cell(CellId(2), payload("checkpoint-b", seed))
            .unwrap();
        let saved = fs::read(dir.path().join("db.aclog")).unwrap();
        db.checkpoint().unwrap();
        saved
    };
    fs::write(dir.path().join("db.aclog"), saved_wal).unwrap();
    assert_visible(
        dir.path(),
        CommitSeq(2),
        &[
            (CellId(1), payload("checkpoint-a", seed)),
            (CellId(2), payload("checkpoint-b", seed)),
        ],
        &[],
    );
}

fn checkpoint_tmp_leftovers(seed: u64) {
    let dir = tempfile::tempdir().unwrap();
    {
        let mut db = Database::open(dir.path()).unwrap();
        db.put_cell(CellId(1), payload("tmp-leftover", seed))
            .unwrap();
        db.checkpoint().unwrap();
    }
    fs::write(dir.path().join("manifest.acm.tmp"), b"interrupted manifest").unwrap();
    fs::write(
        dir.path().join("segments").join("segment-7.acs.tmp"),
        b"interrupted segment",
    )
    .unwrap();
    assert_visible(
        dir.path(),
        CommitSeq(1),
        &[(CellId(1), payload("tmp-leftover", seed))],
        &[CellId(7)],
    );
    assert!(!dir.path().join("manifest.acm.tmp").exists());
}

fn compact_manifest_before_wal_truncate(seed: u64) {
    let dir = tempfile::tempdir().unwrap();
    let saved_wal = {
        let mut db = Database::open(dir.path()).unwrap();
        db.put_cell(CellId(1), payload("compact-base", seed))
            .unwrap();
        db.checkpoint().unwrap();
        db.patch_cell(CellId(1), payload("compact-patch", seed))
            .unwrap();
        db.put_cell(CellId(2), payload("compact-new", seed))
            .unwrap();
        let saved = fs::read(dir.path().join("db.aclog")).unwrap();
        db.compact().unwrap();
        saved
    };
    fs::write(dir.path().join("db.aclog"), saved_wal).unwrap();
    assert_visible(
        dir.path(),
        CommitSeq(3),
        &[
            (CellId(1), payload("compact-patch", seed)),
            (CellId(2), payload("compact-new", seed)),
        ],
        &[],
    );
}

fn partial_wal_tail(seed: u64) {
    let dir = tempfile::tempdir().unwrap();
    {
        let mut db = Database::open(dir.path()).unwrap();
        db.put_cell(CellId(1), payload("partial-wal", seed))
            .unwrap();
    }
    append_partial_tail(&dir.path().join("db.aclog"));
    assert_visible(
        dir.path(),
        CommitSeq(1),
        &[(CellId(1), payload("partial-wal", seed))],
        &[CellId(99)],
    );
}

fn partial_bundle_before_manifest(seed: u64) {
    let dir = base_checkpoint_with_wal_tail(seed, "partial-bundle");
    write_orphan_segment_file_only(dir.path(), 99, CellId(99), seed);
    BitmapIndex::default()
        .write(dir.path().join("segments").join("segment-99.acb"))
        .unwrap();
    assert_visible(
        dir.path(),
        CommitSeq(2),
        &[
            (CellId(1), payload("base", seed)),
            (CellId(2), payload("tail", seed)),
        ],
        &[CellId(99)],
    );
}

fn corrupt_orphan_before_manifest(seed: u64) {
    let dir = base_checkpoint_with_wal_tail(seed, "corrupt-orphan");
    fs::write(
        dir.path().join("segments").join("segment-99.acs"),
        b"ACS1 torn orphan",
    )
    .unwrap();
    assert_visible(
        dir.path(),
        CommitSeq(2),
        &[
            (CellId(1), payload("base", seed)),
            (CellId(2), payload("tail", seed)),
        ],
        &[CellId(99)],
    );
}

fn compact_bundle_before_manifest(seed: u64) {
    let dir = tempfile::tempdir().unwrap();
    {
        let mut db = Database::open(dir.path()).unwrap();
        db.put_cell(CellId(1), payload("old", seed)).unwrap();
        db.checkpoint().unwrap();
        db.patch_cell(CellId(1), payload("new", seed)).unwrap();
        db.checkpoint().unwrap();
    }
    write_orphan_bundle(dir.path(), 42, CellId(42), seed);
    assert_visible(
        dir.path(),
        CommitSeq(2),
        &[(CellId(1), payload("new", seed))],
        &[CellId(42)],
    );
}

fn base_checkpoint_with_wal_tail(seed: u64, label: &'static str) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    {
        let mut db = Database::open(dir.path()).unwrap();
        db.put_cell(CellId(1), payload("base", seed)).unwrap();
        db.checkpoint().unwrap();
        db.put_cell(CellId(2), payload("tail", seed)).unwrap();
        fs::write(
            dir.path().join("last_fault_label.txt"),
            format!("{label}-{seed}"),
        )
        .unwrap();
    }
    dir
}

fn assert_visible(
    root: &Path,
    expected_seq: CommitSeq,
    expected: &[(CellId, Vec<u8>)],
    absent: &[CellId],
) {
    let db = Database::open(root).unwrap();
    assert_eq!(db.current_seq(), expected_seq);
    db.validate_storage().unwrap();
    for (cell_id, payload) in expected {
        assert_eq!(db.get_latest_cell(*cell_id).unwrap(), *payload);
    }
    for cell_id in absent {
        assert_eq!(db.get_latest_cell(*cell_id), None);
    }
}

fn write_orphan_bundle(root: &Path, segment_id: u64, cell_id: CellId, seed: u64) {
    let segments = root.join("segments");
    fs::create_dir_all(&segments).unwrap();
    write_orphan_segment_file_only(root, segment_id, cell_id, seed);
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

fn write_orphan_segment_file_only(root: &Path, segment_id: u64, cell_id: CellId, seed: u64) {
    let segments = root.join("segments");
    fs::create_dir_all(&segments).unwrap();
    SegmentWriter::write(
        segments.join(format!("segment-{segment_id}.acs")),
        &[SegmentCell {
            candidate_id: 999,
            cell_id: cell_id.0,
            created_seq: 999,
            deleted_seq: None,
            payload: payload("orphan", seed),
        }],
    )
    .unwrap();
}

fn append_partial_tail(path: &Path) {
    use std::io::Write;
    let mut file = fs::OpenOptions::new().append(true).open(path).unwrap();
    file.write_all(b"partial-tail-after-crash").unwrap();
    file.sync_all().unwrap();
}

fn corrupt_last_byte(path: &Path) {
    let mut bytes = fs::read(path).unwrap();
    *bytes.last_mut().unwrap() ^= 0xff;
    fs::write(path, bytes).unwrap();
}

fn payload(label: &str, seed: u64) -> Vec<u8> {
    format!("scope=default\nstatus=ready\n{label}-{seed}").into_bytes()
}
