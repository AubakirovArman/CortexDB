use crate::cli_ops as ops;

use super::paths::resolve_backup_passphrase;
use super::DispatchContext;

pub(super) fn backup(
    ctx: DispatchContext<'_>,
    path: String,
    backup_path: String,
) -> Result<String, String> {
    ops::backup(&ctx.resolve_string(&path), &backup_path)
}

pub(super) fn backup_encrypted(
    ctx: DispatchContext<'_>,
    path: String,
    archive_path: String,
    passphrase: Option<String>,
    passphrase_env: Option<String>,
) -> Result<String, String> {
    let passphrase = resolve_backup_passphrase(passphrase, passphrase_env)?;
    ops::backup_encrypted(&ctx.resolve_string(&path), &archive_path, &passphrase)
}

pub(super) fn backup_drill(
    ctx: DispatchContext<'_>,
    path: String,
    backup_path: String,
    restore_path: String,
) -> Result<String, String> {
    ops::backup_drill(&ctx.resolve_string(&path), &backup_path, &restore_path)
}

pub(super) fn backup_verify(backup_path: String) -> Result<String, String> {
    ops::backup_verify(&backup_path)
}

pub(super) fn backup_prune(
    backup_root: String,
    prefix: String,
    keep_latest: usize,
    dry_run: bool,
) -> Result<String, String> {
    ops::backup_prune(&backup_root, &prefix, keep_latest, dry_run)
}

pub(super) fn backup_offsite_stage(
    backup_path: String,
    offsite_root: String,
    backup_id: String,
) -> Result<String, String> {
    ops::backup_offsite_stage(&backup_path, &offsite_root, &backup_id)
}

pub(super) fn restore(
    ctx: DispatchContext<'_>,
    backup_path: String,
    path: String,
    dry_run: bool,
    to_seq: Option<u64>,
) -> Result<String, String> {
    ops::restore(&backup_path, &ctx.resolve_string(&path), dry_run, to_seq)
}

pub(super) fn restore_encrypted(
    ctx: DispatchContext<'_>,
    archive_path: String,
    path: String,
    passphrase: Option<String>,
    passphrase_env: Option<String>,
) -> Result<String, String> {
    let passphrase = resolve_backup_passphrase(passphrase, passphrase_env)?;
    ops::restore_encrypted(&archive_path, &ctx.resolve_string(&path), &passphrase)
}
