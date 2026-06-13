use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use cortex_engine::{
    embedding_coverage_report_from_expected_items, embedding_coverage_report_from_jsonl,
    embedding_retry_ids_from_expected_items, embedding_retry_ids_from_jsonl,
    EmbeddingCoverageConfig, EmbeddingCoverageReport, EmbeddingExpectedItem,
};

#[path = "embedding_coverage_check/args.rs"]
mod args;

use args::Args;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let args = Args::parse(env::args().skip(1))?;
    let expected = load_expected(&args)?;
    let embedding_jsonl = read_to_string(&args.embeddings_path)?;
    let config = EmbeddingCoverageConfig {
        expected_dimension: args.expected_dimension,
        min_coverage_basis_points: args.min_coverage_basis_points,
        expected_model: args.expected_model.clone(),
    };
    let report = match &expected {
        ExpectedInput::Ids(ids) => {
            embedding_coverage_report_from_jsonl(ids.clone(), &embedding_jsonl, config.clone())
        }
        ExpectedInput::Items(items) => embedding_coverage_report_from_expected_items(
            items.clone(),
            &embedding_jsonl,
            config.clone(),
        ),
    };
    if let Some(path) = &args.retry_ids_output_path {
        let retry_ids = match expected {
            ExpectedInput::Ids(ids) => {
                embedding_retry_ids_from_jsonl(ids, &embedding_jsonl, config)
            }
            ExpectedInput::Items(items) => {
                embedding_retry_ids_from_expected_items(items, &embedding_jsonl, config)
            }
        };
        write_lines(path, &retry_ids)?;
    }
    write_report(&report, args.output_path.as_ref())?;
    if report.production_ready {
        Ok(())
    } else {
        Err(format!(
            "embedding coverage check failed: coverage={}bps embedded={}/{} missing={} duplicates={} unexpected={} invalid={} empty_vectors={} dimension_mismatches={} stale={}",
            report.coverage_basis_points,
            report.embedded_items,
            report.total_items,
            report.missing_items,
            report.duplicate_items,
            report.unexpected_items,
            report.invalid_rows,
            report.empty_vector_rows,
            report.dimension_mismatch_rows,
            report.stale_items
        ))
    }
}

enum ExpectedInput {
    Ids(Vec<String>),
    Items(Vec<EmbeddingExpectedItem>),
}

fn load_expected(args: &Args) -> Result<ExpectedInput, String> {
    if let Some(path) = &args.expected_manifest_path {
        return Ok(ExpectedInput::Items(load_expected_manifest(path)?));
    }
    if let Some(path) = &args.expected_ids_path {
        return Ok(ExpectedInput::Ids(
            read_to_string(path)?
                .lines()
                .map(str::trim)
                .filter(|line| !line.is_empty())
                .map(ToOwned::to_owned)
                .collect(),
        ));
    }
    let path = args.uuid_index_path.as_ref().ok_or_else(|| {
        "one of --expected-ids, --expected-manifest, or --uuid-index is required".to_owned()
    })?;
    let value: serde_json::Value = serde_json::from_str(&read_to_string(path)?)
        .map_err(|error| format!("invalid uuid index {}: {error}", path.display()))?;
    let object = value
        .as_object()
        .ok_or_else(|| format!("uuid index {} must be a JSON object", path.display()))?;
    let mut ids = object.keys().cloned().collect::<Vec<_>>();
    ids.sort();
    Ok(ExpectedInput::Ids(ids))
}

fn load_expected_manifest(path: &PathBuf) -> Result<Vec<EmbeddingExpectedItem>, String> {
    read_to_string(path)?
        .lines()
        .enumerate()
        .filter(|(_, line)| !line.trim().is_empty())
        .map(|(index, line)| {
            let item: EmbeddingExpectedItem = serde_json::from_str(line).map_err(|error| {
                format!(
                    "{}:{} invalid expected manifest row: {error}",
                    path.display(),
                    index + 1
                )
            })?;
            if item.doc_id.trim().is_empty() {
                return Err(format!("{}:{} missing doc_id", path.display(), index + 1));
            }
            Ok(item)
        })
        .collect()
}

fn write_report(report: &EmbeddingCoverageReport, output: Option<&PathBuf>) -> Result<(), String> {
    let json = serde_json::to_string_pretty(report)
        .map_err(|error| format!("failed to serialize report: {error}"))?
        + "\n";
    if let Some(path) = output {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
        }
        fs::write(path, &json)
            .map_err(|error| format!("failed to write {}: {error}", path.display()))?;
    }
    print!("{json}");
    Ok(())
}

fn read_to_string(path: &PathBuf) -> Result<String, String> {
    fs::read_to_string(path).map_err(|error| format!("failed to read {}: {error}", path.display()))
}

fn write_lines(path: &PathBuf, lines: &[String]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    }
    let mut content = String::new();
    for line in lines {
        content.push_str(line);
        content.push('\n');
    }
    fs::write(path, content).map_err(|error| format!("failed to write {}: {error}", path.display()))
}
