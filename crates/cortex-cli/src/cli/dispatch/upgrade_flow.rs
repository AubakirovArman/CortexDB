use crate::cli_upgrade as upgrade;

use super::super::args::UpgradeCommand;
use super::DispatchContext;

pub(super) fn upgrade(ctx: DispatchContext<'_>, command: UpgradeCommand) -> Result<String, String> {
    match command {
        UpgradeCommand::Prepare {
            path,
            backup_path,
            drill_restore_path,
        } => upgrade::prepare(
            ctx.resolve(&path).to_str().unwrap(),
            &backup_path,
            &drill_restore_path,
            ctx.json,
        ),
        UpgradeCommand::Validate { path } => {
            upgrade::validate_after_upgrade(ctx.resolve(&path).to_str().unwrap(), ctx.json)
        }
        UpgradeCommand::Rollback {
            backup_path,
            rollback_path,
        } => upgrade::rollback(
            &backup_path,
            ctx.resolve(&rollback_path).to_str().unwrap(),
            ctx.json,
        ),
    }
}

pub(super) fn migrate(
    ctx: DispatchContext<'_>,
    path: String,
    backup_path: String,
    drill_restore_path: String,
) -> Result<String, String> {
    upgrade::migrate(
        ctx.resolve(&path).to_str().unwrap(),
        &backup_path,
        &drill_restore_path,
        ctx.json,
    )
}
