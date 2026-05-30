use clap::{error::ErrorKind, Parser, Subcommand};

use crate::{cli_ann as ann, cli_ingest as ingest, cli_ops as ops};

#[derive(Parser, Debug)]
#[command(name = "cortexdb", version, about = "CortexDB local CLI")]
struct Cli {
    #[arg(
        long,
        global = true,
        help = "Print machine-readable JSON when supported"
    )]
    json: bool,
    #[arg(
        long,
        global = true,
        help = "Tenant realm (subdirectory under realms/)"
    )]
    tenant: Option<String>,
    #[command(subcommand)]
    command: Command,
}

fn resolve_path(path: &str, tenant: Option<&str>) -> std::path::PathBuf {
    let base = std::path::PathBuf::from(path);
    match tenant {
        Some(t) if t != "default" => base.join("realms").join(t),
        _ => base,
    }
}

#[derive(Subcommand, Debug)]
enum Command {
    Demo,
    Doctor {
        path: String,
    },
    Completions {
        #[arg(value_enum)]
        shell: clap_complete::Shell,
    },
    Version,
    Put {
        path: String,
        cell_id: String,
        payload: String,
    },
    Get {
        path: String,
        cell_id: String,
    },
    Tombstone {
        path: String,
        cell_id: String,
    },
    Flush {
        path: String,
    },
    Compact {
        path: String,
    },
    Stats {
        path: String,
    },
    Validate {
        path: String,
    },
    AnnValidate {
        path: String,
    },
    Repair {
        path: String,
    },
    Backup {
        path: String,
        backup_path: String,
    },
    BackupDrill {
        path: String,
        backup_path: String,
        restore_path: String,
    },
    BackupPrune {
        backup_root: String,
        prefix: String,
        keep_latest: usize,
    },
    BackupOffsiteStage {
        backup_path: String,
        offsite_root: String,
        backup_id: String,
    },
    Restore {
        backup_path: String,
        path: String,
    },
    GcRetired {
        path: String,
    },
    WalValidate {
        path: String,
    },
    WalDump {
        path: String,
    },
    WalTruncate {
        path: String,
    },
    ManifestDump {
        path: String,
    },
    ManifestValidate {
        path: String,
    },
    Context {
        path: String,
        scope: String,
        aql: String,
    },
    Remember {
        path: String,
        scope: String,
        aql: String,
    },
    Forget {
        path: String,
        cell_id: String,
    },
    Verify {
        path: String,
        scope: String,
        aql: String,
    },
    Aql {
        path: String,
        scope: String,
        aql: String,
    },
    Search {
        path: String,
        scope: String,
        query: String,
    },
    SearchVector {
        path: String,
        scope: String,
        vector: String,
        #[arg(long)]
        fallback: Option<String>,
        #[arg(long)]
        fallback_scan_cap: Option<usize>,
        #[arg(long)]
        min_recall: Option<String>,
        #[arg(long)]
        max_visited_candidates: Option<usize>,
        #[arg(long)]
        require_slo: bool,
    },
    SearchVectorExact {
        path: String,
        scope: String,
        vector: String,
    },
    SearchVectorEval {
        path: String,
        scope: String,
        vector: String,
        #[arg(long)]
        fallback: Option<String>,
        #[arg(long)]
        fallback_scan_cap: Option<usize>,
        #[arg(long)]
        min_recall: Option<String>,
        #[arg(long)]
        max_visited_candidates: Option<usize>,
        #[arg(long)]
        require_slo: bool,
    },
    SearchExplain {
        path: String,
        scope: String,
        query: String,
        #[arg(long, default_value = "keyword")]
        mode: String,
    },
    Unlock {
        path: String,
        #[arg(long)]
        force: bool,
    },
    LoadFixture {
        path: String,
        fixture_path: String,
    },
    IngestText {
        path: String,
        scope: String,
        file: String,
    },
    IngestJson {
        path: String,
        scope: String,
        file: String,
    },
    IngestCsv {
        path: String,
        scope: String,
        file: String,
    },
}

pub fn run(args: Vec<String>) -> Result<String, String> {
    let cli = match Cli::try_parse_from(args) {
        Ok(cli) => cli,
        Err(err)
            if matches!(
                err.kind(),
                ErrorKind::DisplayHelp | ErrorKind::DisplayVersion
            ) =>
        {
            return Ok(err.to_string());
        }
        Err(err) => return Err(err.to_string()),
    };
    let resolved = |p: &str| resolve_path(p, cli.tenant.as_deref());
    match cli.command {
        Command::Demo => ops::run_demo(),
        Command::Doctor { path } => ops::doctor(resolved(&path).to_str().unwrap()),
        Command::Completions { shell } => {
            let mut cmd = <Cli as clap::CommandFactory>::command();
            let name = cmd.get_name().to_owned();
            clap_complete::generate(shell, &mut cmd, name, &mut std::io::stdout());
            Ok("".to_owned())
        }
        Command::Version => Ok(format!("cortexdb {}", env!("CARGO_PKG_VERSION"))),
        Command::Put {
            path,
            cell_id,
            payload,
        } => ops::put(resolved(&path).to_str().unwrap(), &cell_id, &payload),
        Command::Get { path, cell_id } => {
            ops::get(resolved(&path).to_str().unwrap(), &cell_id, cli.json)
        }
        Command::Tombstone { path, cell_id } => {
            ops::tombstone(resolved(&path).to_str().unwrap(), &cell_id)
        }
        Command::Flush { path } => ops::flush(resolved(&path).to_str().unwrap()),
        Command::Compact { path } => ops::compact(resolved(&path).to_str().unwrap()),
        Command::Stats { path } => ops::stats(resolved(&path).to_str().unwrap(), cli.json),
        Command::Validate { path } => ops::validate(resolved(&path).to_str().unwrap(), cli.json),
        Command::AnnValidate { path } => {
            ops::ann_validate(resolved(&path).to_str().unwrap(), cli.json)
        }
        Command::Repair { path } => ops::repair(resolved(&path).to_str().unwrap()),
        Command::Backup { path, backup_path } => {
            ops::backup(resolved(&path).to_str().unwrap(), &backup_path)
        }
        Command::BackupDrill {
            path,
            backup_path,
            restore_path,
        } => ops::backup_drill(
            resolved(&path).to_str().unwrap(),
            &backup_path,
            &restore_path,
        ),
        Command::BackupPrune {
            backup_root,
            prefix,
            keep_latest,
        } => ops::backup_prune(&backup_root, &prefix, keep_latest),
        Command::BackupOffsiteStage {
            backup_path,
            offsite_root,
            backup_id,
        } => ops::backup_offsite_stage(&backup_path, &offsite_root, &backup_id),
        Command::Restore { backup_path, path } => {
            ops::restore(&backup_path, resolved(&path).to_str().unwrap())
        }
        Command::GcRetired { path } => ops::gc_retired(resolved(&path).to_str().unwrap()),
        Command::WalValidate { path } => ops::wal_validate(resolved(&path).to_str().unwrap()),
        Command::WalDump { path } => ops::wal_dump(resolved(&path).to_str().unwrap()),
        Command::WalTruncate { path } => ops::wal_truncate(resolved(&path).to_str().unwrap()),
        Command::ManifestDump { path } => ops::manifest_dump(resolved(&path).to_str().unwrap()),
        Command::ManifestValidate { path } => {
            ops::manifest_validate(resolved(&path).to_str().unwrap())
        }
        Command::Context { path, scope, aql } => {
            ops::context(resolved(&path).to_str().unwrap(), &scope, &aql, cli.json)
        }
        Command::Remember { path, scope, aql } => {
            ops::remember(resolved(&path).to_str().unwrap(), &scope, &aql, cli.json)
        }
        Command::Forget { path, cell_id } => {
            ops::forget(resolved(&path).to_str().unwrap(), &cell_id, cli.json)
        }
        Command::Verify { path, scope, aql } => {
            ops::verify(resolved(&path).to_str().unwrap(), &scope, &aql, cli.json)
        }
        Command::Aql { path, scope, aql } => {
            ops::aql(resolved(&path).to_str().unwrap(), &scope, &aql, cli.json)
        }
        Command::Search { path, scope, query } => {
            ops::search(resolved(&path).to_str().unwrap(), &scope, &query, cli.json)
        }
        Command::SearchVector {
            path,
            scope,
            vector,
            fallback,
            fallback_scan_cap,
            min_recall,
            max_visited_candidates,
            require_slo,
        } => {
            let policy = ann::parse_ann_policy(
                fallback,
                fallback_scan_cap,
                min_recall,
                max_visited_candidates,
                require_slo,
            )?;
            ops::search_vector(
                resolved(&path).to_str().unwrap(),
                &scope,
                &vector,
                false,
                Some(policy),
            )
        }
        Command::SearchVectorExact {
            path,
            scope,
            vector,
        } => ops::search_vector(
            resolved(&path).to_str().unwrap(),
            &scope,
            &vector,
            true,
            None,
        ),
        Command::SearchVectorEval {
            path,
            scope,
            vector,
            fallback,
            fallback_scan_cap,
            min_recall,
            max_visited_candidates,
            require_slo,
        } => {
            let policy = ann::parse_ann_policy(
                fallback,
                fallback_scan_cap,
                min_recall,
                max_visited_candidates,
                require_slo,
            )?;
            ann::search_vector_eval(
                resolved(&path).to_str().unwrap(),
                &scope,
                &vector,
                cli.json,
                Some(policy),
            )
        }
        Command::SearchExplain {
            path,
            scope,
            query,
            mode,
        } => ops::search_explain(resolved(&path).to_str().unwrap(), &scope, &query, &mode),
        Command::Unlock { path, force } => ops::unlock(resolved(&path).to_str().unwrap(), force),
        Command::LoadFixture { path, fixture_path } => {
            ingest::load_fixture(resolved(&path).to_str().unwrap(), &fixture_path)
        }
        Command::IngestText { path, scope, file } => {
            ingest::text(resolved(&path).to_str().unwrap(), &scope, &file)
        }
        Command::IngestJson { path, scope, file } => {
            ingest::json(resolved(&path).to_str().unwrap(), &scope, &file)
        }
        Command::IngestCsv { path, scope, file } => {
            ingest::csv(resolved(&path).to_str().unwrap(), &scope, &file)
        }
    }
}
