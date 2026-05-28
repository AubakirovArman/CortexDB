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
    #[command(subcommand)]
    command: Command,
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
    match cli.command {
        Command::Demo => ops::run_demo(),
        Command::Doctor { path } => ops::doctor(&path),
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
        } => ops::put(&path, &cell_id, &payload),
        Command::Get { path, cell_id } => ops::get(&path, &cell_id),
        Command::Tombstone { path, cell_id } => ops::tombstone(&path, &cell_id),
        Command::Flush { path } => ops::flush(&path),
        Command::Compact { path } => ops::compact(&path),
        Command::Stats { path } => ops::stats(&path, cli.json),
        Command::Validate { path } => ops::validate(&path, cli.json),
        Command::AnnValidate { path } => ops::ann_validate(&path, cli.json),
        Command::Repair { path } => ops::repair(&path),
        Command::GcRetired { path } => ops::gc_retired(&path),
        Command::WalValidate { path } => ops::wal_validate(&path),
        Command::WalDump { path } => ops::wal_dump(&path),
        Command::WalTruncate { path } => ops::wal_truncate(&path),
        Command::ManifestDump { path } => ops::manifest_dump(&path),
        Command::ManifestValidate { path } => ops::manifest_validate(&path),
        Command::Context { path, scope, aql } => ops::context(&path, &scope, &aql, cli.json),
        Command::Remember { path, scope, aql } => ops::remember(&path, &scope, &aql),
        Command::Verify { path, scope, aql } => ops::verify(&path, &scope, &aql, cli.json),
        Command::Aql { path, scope, aql } => ops::aql(&path, &scope, &aql),
        Command::Search { path, scope, query } => ops::search(&path, &scope, &query),
        Command::SearchVector {
            path,
            scope,
            vector,
        } => ops::search_vector(&path, &scope, &vector, false),
        Command::SearchVectorExact {
            path,
            scope,
            vector,
        } => ops::search_vector(&path, &scope, &vector, true),
        Command::SearchVectorEval {
            path,
            scope,
            vector,
        } => ann::search_vector_eval(&path, &scope, &vector, cli.json),
        Command::Unlock { path, force } => ops::unlock(&path, force),
        Command::LoadFixture { path, fixture_path } => ingest::load_fixture(&path, &fixture_path),
        Command::IngestText { path, scope, file } => ingest::text(&path, &scope, &file),
        Command::IngestJson { path, scope, file } => ingest::json(&path, &scope, &file),
        Command::IngestCsv { path, scope, file } => ingest::csv(&path, &scope, &file),
    }
}
