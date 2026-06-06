use cortex_core::CellId;
use cortex_engine::{
    Database, EngineError, LocalFilesystemOffsiteAdapter, OffsiteBackupAdapter,
    OffsiteBackupTransferReport, StorageValidation,
};
use std::path::{Path, PathBuf};

#[test]
fn backup_restore_preserves_wal_tail_without_checkpoint() {
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("source");
    let backup = root.path().join("backup");
    let target = root.path().join("target");

    let mut db = Database::open(&source).unwrap();
    db.put_cell(CellId(1), b"wal-only payload".to_vec())
        .unwrap();
    let report = db.backup_to(&backup).unwrap();
    assert!(report.files_copied > 0);
    assert!(report.bytes_copied > 0);

    db.put_cell(CellId(2), b"after backup".to_vec()).unwrap();
    drop(db);

    let restored = Database::restore_from_backup(&backup, &target).unwrap();
    assert!(restored.files_copied > 0);
    assert_eq!(restored.restored_validation.wal_records_checked, 1);

    let db = Database::open(&target).unwrap();
    assert_eq!(db.get_latest_cell(CellId(1)).unwrap(), b"wal-only payload");
    assert_eq!(db.get_latest_cell(CellId(2)), None);
}

#[test]
fn backup_restore_preserves_checkpointed_segment() {
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("source");
    let backup = root.path().join("backup");
    let target = root.path().join("target");

    {
        let mut db = Database::open(&source).unwrap();
        db.put_cell(
            CellId(10),
            b"scope=project:investments\nstatus=ready\ncheckpointed".to_vec(),
        )
        .unwrap();
        db.checkpoint().unwrap();
    }

    let backup_report = Database::backup_path(&source, &backup).unwrap();
    assert_eq!(backup_report.source_validation.live_segments_checked, 1);

    let restore_report = Database::restore_from_backup(&backup, &target).unwrap();
    assert_eq!(restore_report.restored_validation.live_segments_checked, 1);

    let db = Database::open(&target).unwrap();
    assert_eq!(
        db.get_latest_cell(CellId(10)).unwrap(),
        b"scope=project:investments\nstatus=ready\ncheckpointed"
    );
    db.validate_storage().unwrap();
}

#[test]
fn backup_restore_drill_proves_recovered_database_is_readable() {
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("source");
    let backup = root.path().join("backup");
    let drill_target = root.path().join("drill-target");

    {
        let mut db = Database::open(&source).unwrap();
        db.put_cell(CellId(7), b"drill wal tail".to_vec()).unwrap();
        db.put_cell(
            CellId(8),
            b"scope=project:ops\nstatus=ready\ndrill checkpointed".to_vec(),
        )
        .unwrap();
        db.checkpoint().unwrap();
        db.put_cell(CellId(9), b"drill post-checkpoint tail".to_vec())
            .unwrap();
    }

    let report = Database::backup_restore_drill_path(&source, &backup, &drill_target).unwrap();
    assert!(report.backup.files_copied > 0);
    assert!(report.restore.files_copied > 0);
    assert_eq!(report.restore.restored_validation.live_segments_checked, 1);
    assert_eq!(report.restore.restored_validation.wal_records_checked, 1);

    let restored = Database::open(&drill_target).unwrap();
    assert_eq!(
        restored.get_latest_cell(CellId(7)).unwrap(),
        b"drill wal tail"
    );
    assert_eq!(
        restored.get_latest_cell(CellId(8)).unwrap(),
        b"scope=project:ops\nstatus=ready\ndrill checkpointed"
    );
    assert_eq!(
        restored.get_latest_cell(CellId(9)).unwrap(),
        b"drill post-checkpoint tail"
    );
}

#[test]
fn restore_rejects_existing_target() {
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("source");
    let backup = root.path().join("backup");
    let target = root.path().join("target");

    {
        let mut db = Database::open(&source).unwrap();
        db.put_cell(CellId(1), b"payload".to_vec()).unwrap();
    }
    Database::backup_path(&source, &backup).unwrap();
    std::fs::create_dir(&target).unwrap();

    assert!(matches!(
        Database::restore_from_backup(&backup, &target).unwrap_err(),
        EngineError::BackupTargetExists(_)
    ));
}

#[test]
fn restore_dry_run_validates_backup_without_creating_target() {
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("source");
    let backup = root.path().join("backup");
    let target = root.path().join("target");

    {
        let mut db = Database::open(&source).unwrap();
        db.put_cell(
            CellId(11),
            b"scope=ops\nstatus=ready\ndry run checkpointed".to_vec(),
        )
        .unwrap();
        db.checkpoint().unwrap();
        db.put_cell(CellId(12), b"dry run wal tail".to_vec())
            .unwrap();
    }
    Database::backup_path(&source, &backup).unwrap();

    let report = Database::restore_from_backup_dry_run(&backup, &target).unwrap();

    assert!(!target.exists());
    assert_eq!(report.restore_path, target);
    assert!(report.files_checked > 0);
    assert!(report.bytes_checked > 0);
    assert!(report.version_compatible);
    assert_eq!(report.backup_validation.live_segments_checked, 1);
    assert_eq!(report.backup_validation.wal_records_checked, 1);
}

#[test]
fn restore_dry_run_rejects_corrupt_backup_without_creating_target() {
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("source");
    let backup = root.path().join("backup");
    let target = root.path().join("target");

    {
        let mut db = Database::open(&source).unwrap();
        db.put_cell(
            CellId(13),
            b"scope=ops\nstatus=ready\ndry run corruption".to_vec(),
        )
        .unwrap();
        db.checkpoint().unwrap();
    }
    Database::backup_path(&source, &backup).unwrap();
    corrupt_first_byte(&find_file_with_extension(&backup, "acs"));

    assert!(Database::restore_from_backup_dry_run(&backup, &target).is_err());
    assert!(!target.exists());
}

#[test]
fn restore_dry_run_rejects_existing_target() {
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("source");
    let backup = root.path().join("backup");
    let target = root.path().join("target");

    {
        let mut db = Database::open(&source).unwrap();
        db.put_cell(CellId(14), b"payload".to_vec()).unwrap();
    }
    Database::backup_path(&source, &backup).unwrap();
    std::fs::create_dir(&target).unwrap();

    assert!(matches!(
        Database::restore_from_backup_dry_run(&backup, &target).unwrap_err(),
        EngineError::BackupTargetExists(_)
    ));
}

#[test]
fn backup_rejects_target_inside_source() {
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("source");
    let backup = source.join("nested-backup");

    let mut db = Database::open(&source).unwrap();
    db.put_cell(CellId(1), b"payload".to_vec()).unwrap();

    let error = db.backup_to(&backup).unwrap_err().to_string();
    assert!(error.contains("backup target must not be inside source database"));
}

#[test]
fn backup_retention_prunes_oldest_matching_backups() {
    let root = tempfile::tempdir().unwrap();
    for name in [
        "cortexdb-20260528T000000Z",
        "cortexdb-20260529T000000Z",
        "cortexdb-20260530T000000Z",
        "other-20260527T000000Z",
    ] {
        let dir = root.path().join(name);
        std::fs::create_dir(&dir).unwrap();
        std::fs::write(dir.join("marker"), name.as_bytes()).unwrap();
    }

    let report = Database::prune_backup_retention(root.path(), "cortexdb-", 2).unwrap();

    assert_eq!(report.backups_seen, 3);
    assert_eq!(report.backups_kept, 2);
    assert_eq!(report.backups_removed, 1);
    assert!(report.bytes_removed > 0);
    assert!(!root.path().join("cortexdb-20260528T000000Z").exists());
    assert!(root.path().join("cortexdb-20260529T000000Z").exists());
    assert!(root.path().join("cortexdb-20260530T000000Z").exists());
    assert!(root.path().join("other-20260527T000000Z").exists());
}

#[test]
fn backup_retention_dry_run_reports_without_deleting() {
    let root = tempfile::tempdir().unwrap();
    for name in [
        "cortexdb-20260528T000000Z",
        "cortexdb-20260529T000000Z",
        "cortexdb-20260530T000000Z",
    ] {
        let dir = root.path().join(name);
        std::fs::create_dir(&dir).unwrap();
        std::fs::write(dir.join("marker"), name.as_bytes()).unwrap();
    }

    let report = Database::prune_backup_retention_dry_run(root.path(), "cortexdb-", 2).unwrap();

    assert!(report.dry_run);
    assert_eq!(report.backups_seen, 3);
    assert_eq!(report.backups_kept, 2);
    assert_eq!(report.backups_removed, 1);
    assert!(report.bytes_removed > 0);
    assert!(root.path().join("cortexdb-20260528T000000Z").exists());
    assert!(root.path().join("cortexdb-20260529T000000Z").exists());
    assert!(root.path().join("cortexdb-20260530T000000Z").exists());
}

#[test]
fn backup_retention_never_deletes_only_matching_backup() {
    let root = tempfile::tempdir().unwrap();
    let backup = root.path().join("cortexdb-20260530T000000Z");
    std::fs::create_dir(&backup).unwrap();
    std::fs::write(backup.join("marker"), b"only").unwrap();

    let report = Database::prune_backup_retention(root.path(), "cortexdb-", 1).unwrap();

    assert!(!report.dry_run);
    assert_eq!(report.backups_seen, 1);
    assert_eq!(report.backups_kept, 1);
    assert_eq!(report.backups_removed, 0);
    assert!(backup.exists());
}

#[test]
fn backup_retention_rejects_unsafe_plan() {
    let root = tempfile::tempdir().unwrap();

    let empty_prefix = Database::prune_backup_retention(root.path(), "", 1)
        .unwrap_err()
        .to_string();
    assert!(empty_prefix.contains("prefix must not be empty"));

    let zero_keep = Database::prune_backup_retention(root.path(), "cortexdb-", 0)
        .unwrap_err()
        .to_string();
    assert!(zero_keep.contains("keep_latest must be greater than zero"));
}

#[test]
fn offsite_stage_validates_backup_and_publishes_copy() {
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("source");
    let backup = root.path().join("backup");
    let offsite = root.path().join("offsite");

    {
        let mut db = Database::open(&source).unwrap();
        db.put_cell(CellId(44), b"offsite payload".to_vec())
            .unwrap();
        db.checkpoint().unwrap();
    }
    Database::backup_path(&source, &backup).unwrap();

    let report =
        Database::stage_backup_offsite(&backup, &offsite, "cortexdb-20260530T000000Z").unwrap();

    assert!(report.target_path.exists());
    assert_eq!(report.adapter, "local_filesystem");
    assert!(report.published);
    assert!(report.files_copied > 0);
    assert!(report.bytes_copied > 0);
    assert_eq!(report.drill_restore.restored_validation.cells_checked, 1);
    assert_eq!(report.staged_validation.cells_checked, 1);
    assert!(!offsite.join("cortexdb-20260530T000000Z.staging").exists());
    assert!(!offsite
        .join("cortexdb-20260530T000000Z.preflight-restore")
        .exists());

    let restored = Database::open(offsite.join("cortexdb-20260530T000000Z")).unwrap();
    assert_eq!(
        restored.get_latest_cell(CellId(44)).unwrap(),
        b"offsite payload"
    );
}

#[test]
fn offsite_adapter_validation_failure_removes_staging_copy() {
    struct RejectingValidationAdapter;

    impl OffsiteBackupAdapter for RejectingValidationAdapter {
        fn name(&self) -> &'static str {
            "rejecting_validation_test"
        }

        fn stage_backup(
            &self,
            backup_path: &Path,
            staging_path: &Path,
        ) -> cortex_engine::EngineResult<OffsiteBackupTransferReport> {
            LocalFilesystemOffsiteAdapter.stage_backup(backup_path, staging_path)
        }

        fn validate_staged_backup(
            &self,
            _staging_path: &Path,
        ) -> cortex_engine::EngineResult<StorageValidation> {
            Err(EngineError::StorageInvariant(
                "adapter validation failed".to_owned(),
            ))
        }

        fn publish_staged_backup(
            &self,
            _staging_path: &Path,
            _final_path: &Path,
        ) -> cortex_engine::EngineResult<()> {
            unreachable!("validation failure must stop before publish")
        }
    }

    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("source");
    let backup = root.path().join("backup");
    let offsite = root.path().join("offsite");

    {
        let mut db = Database::open(&source).unwrap();
        db.put_cell(CellId(46), b"payload".to_vec()).unwrap();
    }
    Database::backup_path(&source, &backup).unwrap();

    let error = Database::stage_backup_offsite_with_adapter(
        &backup,
        &offsite,
        "backup-adapter-fails",
        &RejectingValidationAdapter,
    )
    .unwrap_err()
    .to_string();

    assert!(error.contains("adapter validation failed"));
    assert!(!offsite.join("backup-adapter-fails.staging").exists());
    assert!(!offsite.join("backup-adapter-fails").exists());
}

#[test]
fn offsite_stage_rejects_unsafe_id_and_existing_target() {
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("source");
    let backup = root.path().join("backup");
    let offsite = root.path().join("offsite");

    {
        let mut db = Database::open(&source).unwrap();
        db.put_cell(CellId(45), b"payload".to_vec()).unwrap();
    }
    Database::backup_path(&source, &backup).unwrap();

    let unsafe_id = Database::stage_backup_offsite(&backup, &offsite, "../bad")
        .unwrap_err()
        .to_string();
    assert!(unsafe_id.contains("offsite backup id"));

    Database::stage_backup_offsite(&backup, &offsite, "backup-a").unwrap();
    assert!(matches!(
        Database::stage_backup_offsite(&backup, &offsite, "backup-a").unwrap_err(),
        EngineError::BackupTargetExists(_)
    ));
}

#[test]
fn corrupt_backup_segment_archive_is_rejected_on_restore() {
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("source");
    let backup = root.path().join("backup");
    let target = root.path().join("target");

    {
        let mut db = Database::open(&source).unwrap();
        db.put_cell(
            CellId(81),
            b"scope=ops\nstatus=ready\ncorruption test".to_vec(),
        )
        .unwrap();
        db.checkpoint().unwrap();
    }
    Database::backup_path(&source, &backup).unwrap();

    corrupt_first_byte(&find_file_with_extension(&backup, "acs"));

    assert!(
        Database::restore_from_backup(&backup, &target).is_err(),
        "restore must reject backup archive with corrupted segment file"
    );
}

#[test]
fn corrupt_backup_manifest_archive_is_rejected_on_restore() {
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("source");
    let backup = root.path().join("backup");
    let target = root.path().join("target");

    {
        let mut db = Database::open(&source).unwrap();
        db.put_cell(
            CellId(82),
            b"scope=ops\nstatus=ready\nmanifest corruption test".to_vec(),
        )
        .unwrap();
        db.checkpoint().unwrap();
    }
    Database::backup_path(&source, &backup).unwrap();

    corrupt_first_byte(&backup.join("manifest.acm"));

    assert!(
        Database::restore_from_backup(&backup, &target).is_err(),
        "restore must reject backup archive with corrupted manifest"
    );
}

#[test]
fn encrypted_backup_restore_roundtrips_wal_and_checkpointed_data() {
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("source");
    let archive = root.path().join("backup.cdbenc");
    let target = root.path().join("target");
    let passphrase = "correct horse battery staple";

    {
        let mut db = Database::open(&source).unwrap();
        db.put_cell(
            CellId(91),
            b"scope=ops\nstatus=ready\nencrypted checkpointed".to_vec(),
        )
        .unwrap();
        db.checkpoint().unwrap();
        db.put_cell(CellId(92), b"encrypted wal tail".to_vec())
            .unwrap();
    }

    let backup = Database::encrypted_backup_path(&source, &archive, passphrase).unwrap();
    assert!(backup.files_archived > 0);
    assert!(backup.ciphertext_bytes > 0);
    assert!(archive.exists());
    let raw_archive = std::fs::read(&archive).unwrap();
    assert!(!raw_archive
        .windows("encrypted wal tail".len())
        .any(|window| window == b"encrypted wal tail"));

    let restore = Database::restore_from_encrypted_backup(&archive, &target, passphrase).unwrap();
    assert!(restore.files_restored > 0);
    assert_eq!(restore.restored_validation.live_segments_checked, 1);
    assert_eq!(restore.restored_validation.wal_records_checked, 1);

    let db = Database::open(&target).unwrap();
    assert_eq!(
        db.get_latest_cell(CellId(91)).unwrap(),
        b"scope=ops\nstatus=ready\nencrypted checkpointed"
    );
    assert_eq!(
        db.get_latest_cell(CellId(92)).unwrap(),
        b"encrypted wal tail"
    );
}

#[test]
fn encrypted_backup_wrong_passphrase_fails_without_target() {
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("source");
    let archive = root.path().join("backup.cdbenc");
    let target = root.path().join("target");

    {
        let mut db = Database::open(&source).unwrap();
        db.put_cell(CellId(93), b"secret payload".to_vec()).unwrap();
    }
    Database::encrypted_backup_path(&source, &archive, "correct passphrase 123").unwrap();

    let error = Database::restore_from_encrypted_backup(&archive, &target, "wrong passphrase 123")
        .unwrap_err()
        .to_string();

    assert!(error.contains("passphrase") || error.contains("authentication"));
    assert!(!target.exists());
}

#[test]
fn encrypted_backup_corrupt_ciphertext_is_rejected() {
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("source");
    let archive = root.path().join("backup.cdbenc");
    let target = root.path().join("target");
    let passphrase = "correct passphrase 456";

    {
        let mut db = Database::open(&source).unwrap();
        db.put_cell(CellId(94), b"payload".to_vec()).unwrap();
    }
    Database::encrypted_backup_path(&source, &archive, passphrase).unwrap();
    let mut bytes = std::fs::read(&archive).unwrap();
    let last = bytes.len() - 1;
    bytes[last] ^= 0x7f;
    std::fs::write(&archive, bytes).unwrap();

    let error = Database::restore_from_encrypted_backup(&archive, &target, passphrase)
        .unwrap_err()
        .to_string();

    assert!(error.contains("ciphertext checksum"));
    assert!(!target.exists());
}

fn find_file_with_extension(root: &Path, extension: &str) -> PathBuf {
    for entry in std::fs::read_dir(root).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_dir() {
            let found = find_file_with_extension(&path, extension);
            if found.exists() {
                return found;
            }
        } else if path
            .extension()
            .is_some_and(|value| value == std::ffi::OsStr::new(extension))
        {
            return path;
        }
    }
    root.join(format!("missing.{extension}"))
}

fn corrupt_first_byte(path: &Path) {
    let mut bytes = std::fs::read(path).unwrap();
    bytes[0] ^= 0xFF;
    std::fs::write(path, bytes).unwrap();
}
