use cortex_engine::Database;

use super::common::fmt_engine_error;

pub fn backup(path: &str, backup_path: &str) -> Result<String, String> {
    let report = Database::backup_path(path, backup_path).map_err(fmt_engine_error)?;
    Ok(format!(
        "files_copied={} bytes_copied={} source_live_segments_checked={} source_cells_checked={} source_wal_records_checked={} checksum_manifest_files={}",
        report.files_copied,
        report.bytes_copied,
        report.source_validation.live_segments_checked,
        report.source_validation.cells_checked,
        report.source_validation.wal_records_checked,
        report.checksum_manifest_files
    ))
}

pub fn backup_encrypted(
    path: &str,
    archive_path: &str,
    passphrase: &str,
) -> Result<String, String> {
    let report = Database::encrypted_backup_path(path, archive_path, passphrase)
        .map_err(fmt_engine_error)?;
    Ok(format!(
        "files_archived={} plaintext_bytes={} ciphertext_bytes={} source_live_segments_checked={} source_cells_checked={} source_wal_records_checked={}",
        report.files_archived,
        report.plaintext_bytes,
        report.ciphertext_bytes,
        report.source_validation.live_segments_checked,
        report.source_validation.cells_checked,
        report.source_validation.wal_records_checked
    ))
}

pub fn restore(backup_path: &str, path: &str, dry_run: bool) -> Result<String, String> {
    if dry_run {
        let report =
            Database::restore_from_backup_dry_run(backup_path, path).map_err(fmt_engine_error)?;
        return Ok(format!(
            "dry_run=true restore_path={} files_checked={} bytes_checked={} version_compatible={} checksum_manifest_present={} checksum_manifest_files_verified={} backup_live_segments_checked={} backup_cells_checked={} backup_wal_records_checked={}",
            report.restore_path.display(),
            report.files_checked,
            report.bytes_checked,
            report.version_compatible,
            report.checksum_manifest_present,
            report.checksum_manifest_files_verified,
            report.backup_validation.live_segments_checked,
            report.backup_validation.cells_checked,
            report.backup_validation.wal_records_checked
        ));
    }
    let report = Database::restore_from_backup(backup_path, path).map_err(fmt_engine_error)?;
    Ok(format!(
        "files_copied={} bytes_copied={} restored_live_segments_checked={} restored_cells_checked={} restored_wal_records_checked={}",
        report.files_copied,
        report.bytes_copied,
        report.restored_validation.live_segments_checked,
        report.restored_validation.cells_checked,
        report.restored_validation.wal_records_checked
    ))
}

pub fn restore_encrypted(
    archive_path: &str,
    path: &str,
    passphrase: &str,
) -> Result<String, String> {
    let report = Database::restore_from_encrypted_backup(archive_path, path, passphrase)
        .map_err(fmt_engine_error)?;
    Ok(format!(
        "files_restored={} plaintext_bytes={} ciphertext_bytes={} restored_live_segments_checked={} restored_cells_checked={} restored_wal_records_checked={}",
        report.files_restored,
        report.plaintext_bytes,
        report.ciphertext_bytes,
        report.restored_validation.live_segments_checked,
        report.restored_validation.cells_checked,
        report.restored_validation.wal_records_checked
    ))
}

pub fn backup_drill(path: &str, backup_path: &str, restore_path: &str) -> Result<String, String> {
    let report = Database::backup_restore_drill_path(path, backup_path, restore_path)
        .map_err(fmt_engine_error)?;
    Ok(format!(
        "backup_files_copied={} backup_bytes_copied={} restored_files_copied={} restored_bytes_copied={} restored_live_segments_checked={} restored_cells_checked={} restored_wal_records_checked={}",
        report.backup.files_copied,
        report.backup.bytes_copied,
        report.restore.files_copied,
        report.restore.bytes_copied,
        report.restore.restored_validation.live_segments_checked,
        report.restore.restored_validation.cells_checked,
        report.restore.restored_validation.wal_records_checked
    ))
}

pub fn backup_verify(backup_path: &str) -> Result<String, String> {
    let report = Database::verify_backup_path(backup_path).map_err(fmt_engine_error)?;
    Ok(format!(
        "backup_ok=true files_checked={} bytes_checked={} version_compatible={} checksum_manifest_present={} checksum_manifest_files_verified={} backup_live_segments_checked={} backup_cells_checked={} backup_wal_records_checked={}",
        report.files_checked,
        report.bytes_checked,
        report.version_compatible,
        report.checksum_manifest_present,
        report.checksum_manifest_files_verified,
        report.backup_validation.live_segments_checked,
        report.backup_validation.cells_checked,
        report.backup_validation.wal_records_checked
    ))
}

pub fn backup_prune(
    backup_root: &str,
    prefix: &str,
    keep_latest: usize,
    dry_run: bool,
) -> Result<String, String> {
    let report = if dry_run {
        Database::prune_backup_retention_dry_run(backup_root, prefix, keep_latest)
    } else {
        Database::prune_backup_retention(backup_root, prefix, keep_latest)
    }
    .map_err(fmt_engine_error)?;
    Ok(format!(
        "dry_run={} backups_seen={} backups_kept={} backups_removed={} bytes_removed={}",
        report.dry_run,
        report.backups_seen,
        report.backups_kept,
        report.backups_removed,
        report.bytes_removed
    ))
}

pub fn backup_offsite_stage(
    backup_path: &str,
    offsite_root: &str,
    backup_id: &str,
) -> Result<String, String> {
    let report = Database::stage_backup_offsite(backup_path, offsite_root, backup_id)
        .map_err(fmt_engine_error)?;
    Ok(format!(
        "adapter={} target_path={} published={} files_copied={} bytes_copied={} drill_restored_files_copied={} drill_restored_cells_checked={} staged_live_segments_checked={} staged_cells_checked={} staged_wal_records_checked={}",
        report.adapter,
        report.target_path.display(),
        report.published,
        report.files_copied,
        report.bytes_copied,
        report.drill_restore.files_copied,
        report.drill_restore.restored_validation.cells_checked,
        report.staged_validation.live_segments_checked,
        report.staged_validation.cells_checked,
        report.staged_validation.wal_records_checked
    ))
}
