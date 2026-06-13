use serde::Serialize;

#[derive(Serialize)]
pub(super) struct UpgradePrepareResponse {
    pub(super) phase: &'static str,
    pub(super) status: &'static str,
    pub(super) source_path: String,
    pub(super) backup_path: String,
    pub(super) drill_restore_path: String,
    pub(super) preflight_live_segments_checked: usize,
    pub(super) preflight_cells_checked: usize,
    pub(super) preflight_wal_records_checked: usize,
    pub(super) backup_files_copied: usize,
    pub(super) backup_bytes_copied: u64,
    pub(super) drill_restored_cells_checked: usize,
    pub(super) validate_after_upgrade_command: String,
    pub(super) rollback_command: String,
}

#[derive(Serialize)]
pub(super) struct MigrationOfflineResponse {
    pub(super) phase: &'static str,
    pub(super) status: &'static str,
    pub(super) dry_run: bool,
    pub(super) planned_steps: &'static [&'static str],
    pub(super) source_path: String,
    pub(super) backup_path: String,
    pub(super) drill_restore_path: String,
    pub(super) preflight_live_segments_checked: usize,
    pub(super) preflight_cells_checked: usize,
    pub(super) preflight_wal_records_checked: usize,
    pub(super) backup_files_copied: usize,
    pub(super) backup_bytes_copied: u64,
    pub(super) drill_restored_cells_checked: usize,
    pub(super) migration_segment_id: Option<u64>,
    pub(super) migration_cells_rewritten: usize,
    pub(super) migration_checkpoint_seq: u64,
    pub(super) post_migration_live_segments_checked: usize,
    pub(super) post_migration_cells_checked: usize,
    pub(super) post_migration_wal_records_checked: usize,
    pub(super) current_seq: u64,
    pub(super) checkpoint_seq: u64,
    pub(super) validate_after_migration_command: String,
    pub(super) rollback_command: String,
}

pub(super) const MIGRATION_OFFLINE_STEPS: &[&str] = &[
    "validate_source",
    "backup_restore_drill",
    "rewrite_checkpoint_segments",
    "validate_after_migration",
    "rollback_restore_available",
];

#[derive(Serialize)]
pub(super) struct UpgradeValidateResponse {
    pub(super) phase: &'static str,
    pub(super) status: &'static str,
    pub(super) path: String,
    pub(super) live_segments_checked: usize,
    pub(super) cells_checked: usize,
    pub(super) wal_records_checked: usize,
    pub(super) current_seq: u64,
    pub(super) checkpoint_seq: u64,
}

#[derive(Serialize)]
pub(super) struct UpgradeRollbackResponse {
    pub(super) phase: &'static str,
    pub(super) status: &'static str,
    pub(super) backup_path: String,
    pub(super) rollback_path: String,
    pub(super) dry_run_files_checked: usize,
    pub(super) dry_run_bytes_checked: u64,
    pub(super) files_copied: usize,
    pub(super) bytes_copied: u64,
    pub(super) restored_cells_checked: usize,
    pub(super) restored_wal_records_checked: usize,
    pub(super) start_previous_binary_against: String,
}

pub(super) fn to_json<T: Serialize>(value: &T) -> String {
    serde_json::to_string(value).unwrap_or_else(|error| {
        serde_json::json!({
            "status": "failed",
            "error": error.to_string(),
        })
        .to_string()
    })
}
