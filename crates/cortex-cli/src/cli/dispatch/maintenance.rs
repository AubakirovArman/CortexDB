use crate::{cli_ann as ann, cli_ops as ops};

use super::super::args::{Command, VectorCommand};
use super::DispatchContext;

pub(super) fn run(ctx: DispatchContext<'_>, command: Command) -> Result<String, String> {
    match command {
        Command::Demo => ops::run_demo(),
        Command::Doctor { path } => ops::doctor(ctx.resolve(&path).to_str().unwrap(), ctx.tenant),
        Command::Put {
            path,
            cell_id,
            payload,
        } => ops::put(ctx.resolve(&path).to_str().unwrap(), &cell_id, &payload),
        Command::Get { path, cell_id } => {
            ops::get(ctx.resolve(&path).to_str().unwrap(), &cell_id, ctx.json)
        }
        Command::Tombstone { path, cell_id } => {
            ops::tombstone(ctx.resolve(&path).to_str().unwrap(), &cell_id)
        }
        Command::Flush {
            path,
            experimental_hnsw,
        } => ops::flush(ctx.resolve(&path).to_str().unwrap(), experimental_hnsw),
        Command::Compact {
            path,
            experimental_hnsw,
        } => ops::compact(ctx.resolve(&path).to_str().unwrap(), experimental_hnsw),
        Command::Stats { path } => ops::stats(ctx.resolve(&path).to_str().unwrap(), ctx.json),
        Command::Validate { path } => ops::validate(ctx.resolve(&path).to_str().unwrap(), ctx.json),
        Command::Vector { command } => match command {
            VectorCommand::Rebuild {
                path,
                experimental_hnsw,
            } => ops::vector_rebuild(
                ctx.resolve(&path).to_str().unwrap(),
                experimental_hnsw,
                ctx.json,
            ),
        },
        Command::AnnValidate { path } => {
            ops::ann_validate(ctx.resolve(&path).to_str().unwrap(), ctx.json)
        }
        Command::HnswNoFallbackProfileShow { path } => {
            ops::hnsw_no_fallback_profile_show(ctx.resolve(&path).to_str().unwrap(), ctx.json)
        }
        Command::HnswNoFallbackProfileSet {
            path,
            enabled,
            min_recall,
            require_upper_layers,
        } => {
            let policy = ann::parse_no_fallback_profile(enabled, min_recall, require_upper_layers)?;
            ops::hnsw_no_fallback_profile_set(
                ctx.resolve(&path).to_str().unwrap(),
                policy,
                ctx.json,
            )
        }
        Command::HnswNoFallbackProfileClear { path } => {
            ops::hnsw_no_fallback_profile_clear(ctx.resolve(&path).to_str().unwrap(), ctx.json)
        }
        Command::Repair { path, dry_run } => {
            ops::repair(ctx.resolve(&path).to_str().unwrap(), dry_run)
        }
        Command::GcRetired { path } => ops::gc_retired(ctx.resolve(&path).to_str().unwrap()),
        Command::WalValidate { path } => ops::wal_validate(ctx.resolve(&path).to_str().unwrap()),
        Command::WalDump { path } => ops::wal_dump(ctx.resolve(&path).to_str().unwrap()),
        Command::WalTruncate { path } => ops::wal_truncate(ctx.resolve(&path).to_str().unwrap()),
        Command::ManifestDump { path } => ops::manifest_dump(ctx.resolve(&path).to_str().unwrap()),
        Command::ManifestValidate { path } => {
            ops::manifest_validate(ctx.resolve(&path).to_str().unwrap())
        }
        Command::Unlock { path, force } => ops::unlock(ctx.resolve(&path).to_str().unwrap(), force),
        _ => unreachable!("non-maintenance command should be handled before maintenance dispatch"),
    }
}
