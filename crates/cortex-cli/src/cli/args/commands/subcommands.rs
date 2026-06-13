use clap::Subcommand;

#[derive(Subcommand, Debug)]
pub(in crate::cli) enum VectorCommand {
    #[command(about = "Rebuild persisted vector and HNSW artifacts")]
    Rebuild {
        path: String,
        #[arg(long)]
        experimental_hnsw: bool,
    },
}

#[derive(Subcommand, Debug)]
pub(in crate::cli) enum UpgradeCommand {
    #[command(about = "Create and validate a pre-upgrade backup drill")]
    Prepare {
        path: String,
        backup_path: String,
        drill_restore_path: String,
    },
    #[command(about = "Validate a database after installing a new binary")]
    Validate { path: String },
    #[command(about = "Restore a pre-upgrade backup into a rollback path")]
    Rollback {
        backup_path: String,
        rollback_path: String,
    },
}
