use cortex_engine::Database;

use crate::cli_ops::{fmt_engine_error, open_database};

mod responses;

use responses::{
    to_json, MigrationOfflineResponse, UpgradePrepareResponse, UpgradeRollbackResponse,
    UpgradeValidateResponse, MIGRATION_OFFLINE_STEPS,
};

pub(crate) fn prepare(
    path: &str,
    backup_path: &str,
    drill_restore_path: &str,
    json: bool,
) -> Result<String, String> {
    let validation = {
        let db = open_database(path, false)?;
        db.validate_storage().map_err(fmt_engine_error)?
    };
    let drill = Database::backup_restore_drill_path(path, backup_path, drill_restore_path)
        .map_err(fmt_engine_error)?;
    let response = UpgradePrepareResponse {
        phase: "upgrade_prepare",
        status: "ready_for_offline_upgrade",
        source_path: path.to_owned(),
        backup_path: backup_path.to_owned(),
        drill_restore_path: drill_restore_path.to_owned(),
        preflight_live_segments_checked: validation.live_segments_checked,
        preflight_cells_checked: validation.cells_checked,
        preflight_wal_records_checked: validation.wal_records_checked,
        backup_files_copied: drill.backup.files_copied,
        backup_bytes_copied: drill.backup.bytes_copied,
        drill_restored_cells_checked: drill.restore.restored_validation.cells_checked,
        validate_after_upgrade_command: format!("cortexdb upgrade validate {path}"),
        rollback_command: format!("cortexdb upgrade rollback {backup_path} {path}.rollback"),
    };
    if json {
        return Ok(to_json(&response));
    }
    Ok(format!(
        "phase={} status={} preflight_live_segments_checked={} preflight_cells_checked={} preflight_wal_records_checked={} backup_files_copied={} backup_bytes_copied={} drill_restored_cells_checked={} validate_after_upgrade_command=\"{}\" rollback_command=\"{}\"",
        response.phase,
        response.status,
        response.preflight_live_segments_checked,
        response.preflight_cells_checked,
        response.preflight_wal_records_checked,
        response.backup_files_copied,
        response.backup_bytes_copied,
        response.drill_restored_cells_checked,
        response.validate_after_upgrade_command,
        response.rollback_command
    ))
}

pub(crate) fn migrate(
    path: &str,
    backup_path: &str,
    drill_restore_path: &str,
    dry_run: bool,
    json: bool,
) -> Result<String, String> {
    let (validation, pre_stats) = {
        let db = open_database(path, false)?;
        let validation = db.validate_storage().map_err(fmt_engine_error)?;
        let stats = db.storage_stats().map_err(fmt_engine_error)?;
        (validation, stats)
    };
    let drill = Database::backup_restore_drill_path(path, backup_path, drill_restore_path)
        .map_err(fmt_engine_error)?;
    let (
        status,
        migration_segment_id,
        migration_cells_rewritten,
        migration_checkpoint_seq,
        post_validation,
        post_stats,
    ) = if dry_run {
        (
            "dry_run_ready",
            None,
            0,
            pre_stats.checkpoint_seq.0,
            validation,
            pre_stats,
        )
    } else {
        let mut db = open_database(path, false)?;
        let migration = db.compact().map_err(fmt_engine_error)?;
        let post_validation = db.validate_storage().map_err(fmt_engine_error)?;
        let post_stats = db.storage_stats().map_err(fmt_engine_error)?;
        (
            "offline_migration_completed",
            migration.segment_id,
            migration.cells_flushed,
            migration.checkpoint_seq.0,
            post_validation,
            post_stats,
        )
    };
    let response = MigrationOfflineResponse {
        phase: "migrate_offline",
        status,
        dry_run,
        planned_steps: MIGRATION_OFFLINE_STEPS,
        source_path: path.to_owned(),
        backup_path: backup_path.to_owned(),
        drill_restore_path: drill_restore_path.to_owned(),
        preflight_live_segments_checked: validation.live_segments_checked,
        preflight_cells_checked: validation.cells_checked,
        preflight_wal_records_checked: validation.wal_records_checked,
        backup_files_copied: drill.backup.files_copied,
        backup_bytes_copied: drill.backup.bytes_copied,
        drill_restored_cells_checked: drill.restore.restored_validation.cells_checked,
        migration_segment_id,
        migration_cells_rewritten,
        migration_checkpoint_seq,
        post_migration_live_segments_checked: post_validation.live_segments_checked,
        post_migration_cells_checked: post_validation.cells_checked,
        post_migration_wal_records_checked: post_validation.wal_records_checked,
        current_seq: post_stats.current_seq.0,
        checkpoint_seq: post_stats.checkpoint_seq.0,
        validate_after_migration_command: format!("cortexdb upgrade validate {path}"),
        rollback_command: format!("cortexdb upgrade rollback {backup_path} {path}.rollback"),
    };
    if json {
        return Ok(to_json(&response));
    }
    Ok(format!(
        "phase={} status={} dry_run={} planned_steps={} preflight_live_segments_checked={} preflight_cells_checked={} preflight_wal_records_checked={} backup_files_copied={} backup_bytes_copied={} drill_restored_cells_checked={} migration_segment_id={:?} migration_cells_rewritten={} migration_checkpoint_seq={} post_migration_live_segments_checked={} post_migration_cells_checked={} post_migration_wal_records_checked={} current_seq={} checkpoint_seq={} validate_after_migration_command=\"{}\" rollback_command=\"{}\"",
        response.phase,
        response.status,
        response.dry_run,
        response.planned_steps.join(","),
        response.preflight_live_segments_checked,
        response.preflight_cells_checked,
        response.preflight_wal_records_checked,
        response.backup_files_copied,
        response.backup_bytes_copied,
        response.drill_restored_cells_checked,
        response.migration_segment_id,
        response.migration_cells_rewritten,
        response.migration_checkpoint_seq,
        response.post_migration_live_segments_checked,
        response.post_migration_cells_checked,
        response.post_migration_wal_records_checked,
        response.current_seq,
        response.checkpoint_seq,
        response.validate_after_migration_command,
        response.rollback_command
    ))
}

pub(crate) fn validate_after_upgrade(path: &str, json: bool) -> Result<String, String> {
    let db = open_database(path, false)?;
    let validation = db.validate_storage().map_err(fmt_engine_error)?;
    let stats = db.storage_stats().map_err(fmt_engine_error)?;
    let response = UpgradeValidateResponse {
        phase: "upgrade_validate",
        status: "validated_after_upgrade",
        path: path.to_owned(),
        live_segments_checked: validation.live_segments_checked,
        cells_checked: validation.cells_checked,
        wal_records_checked: validation.wal_records_checked,
        current_seq: stats.current_seq.0,
        checkpoint_seq: stats.checkpoint_seq.0,
    };
    if json {
        return Ok(to_json(&response));
    }
    Ok(format!(
        "phase={} status={} live_segments_checked={} cells_checked={} wal_records_checked={} current_seq={} checkpoint_seq={}",
        response.phase,
        response.status,
        response.live_segments_checked,
        response.cells_checked,
        response.wal_records_checked,
        response.current_seq,
        response.checkpoint_seq
    ))
}

pub(crate) fn rollback(
    backup_path: &str,
    rollback_path: &str,
    json: bool,
) -> Result<String, String> {
    let dry_run = Database::restore_from_backup_dry_run(backup_path, rollback_path)
        .map_err(fmt_engine_error)?;
    let restore =
        Database::restore_from_backup(backup_path, rollback_path).map_err(fmt_engine_error)?;
    let response = UpgradeRollbackResponse {
        phase: "upgrade_rollback",
        status: "rollback_restored_and_validated",
        backup_path: backup_path.to_owned(),
        rollback_path: rollback_path.to_owned(),
        dry_run_files_checked: dry_run.files_checked,
        dry_run_bytes_checked: dry_run.bytes_checked,
        files_copied: restore.files_copied,
        bytes_copied: restore.bytes_copied,
        restored_cells_checked: restore.restored_validation.cells_checked,
        restored_wal_records_checked: restore.restored_validation.wal_records_checked,
        start_previous_binary_against: rollback_path.to_owned(),
    };
    if json {
        return Ok(to_json(&response));
    }
    Ok(format!(
        "phase={} status={} dry_run_files_checked={} dry_run_bytes_checked={} files_copied={} bytes_copied={} restored_cells_checked={} restored_wal_records_checked={} start_previous_binary_against=\"{}\"",
        response.phase,
        response.status,
        response.dry_run_files_checked,
        response.dry_run_bytes_checked,
        response.files_copied,
        response.bytes_copied,
        response.restored_cells_checked,
        response.restored_wal_records_checked,
        response.start_previous_binary_against
    ))
}
