use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use cortex_engine::{
    embedding_coverage_report_from_expected_items, embedding_coverage_report_from_jsonl,
    embedding_retry_ids_from_expected_items, embedding_retry_ids_from_jsonl,
    EmbeddingCoverageConfig, EmbeddingCoverageReport, EmbeddingExpectedItem,
};

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

#[derive(Debug, PartialEq, Eq)]
struct Args {
    expected_ids_path: Option<PathBuf>,
    expected_manifest_path: Option<PathBuf>,
    uuid_index_path: Option<PathBuf>,
    embeddings_path: PathBuf,
    output_path: Option<PathBuf>,
    retry_ids_output_path: Option<PathBuf>,
    expected_dimension: Option<usize>,
    expected_model: Option<String>,
    min_coverage_basis_points: u32,
}

impl Args {
    fn parse(args: impl IntoIterator<Item = String>) -> Result<Self, String> {
        let mut expected_ids_path = None;
        let mut expected_manifest_path = None;
        let mut uuid_index_path = None;
        let mut embeddings_path = None;
        let mut output_path = None;
        let mut retry_ids_output_path = None;
        let mut expected_dimension = None;
        let mut expected_model = None;
        let mut min_coverage_basis_points = 9_950;
        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--expected-ids" => {
                    expected_ids_path = Some(PathBuf::from(next_value(&mut args, &arg)?));
                }
                "--expected-manifest" => {
                    expected_manifest_path = Some(PathBuf::from(next_value(&mut args, &arg)?));
                }
                "--uuid-index" => {
                    uuid_index_path = Some(PathBuf::from(next_value(&mut args, &arg)?));
                }
                "--embeddings" => {
                    embeddings_path = Some(PathBuf::from(next_value(&mut args, &arg)?));
                }
                "--output" => output_path = Some(PathBuf::from(next_value(&mut args, &arg)?)),
                "--retry-ids-output" => {
                    retry_ids_output_path = Some(PathBuf::from(next_value(&mut args, &arg)?));
                }
                "--expected-dimension" => {
                    expected_dimension = Some(parse_usize(&next_value(&mut args, &arg)?, &arg)?);
                }
                "--expected-model" => {
                    expected_model = Some(next_value(&mut args, &arg)?);
                }
                "--min-coverage-bps" => {
                    min_coverage_basis_points = parse_u32(&next_value(&mut args, &arg)?, &arg)?;
                }
                "--help" | "-h" => return Err(usage()),
                value => return Err(format!("unknown option: {value}")),
            }
        }
        let expected_source_count = usize::from(expected_ids_path.is_some())
            + usize::from(expected_manifest_path.is_some())
            + usize::from(uuid_index_path.is_some());
        if expected_source_count > 1 {
            return Err(
                "use only one of --expected-ids, --expected-manifest, or --uuid-index".to_owned(),
            );
        }
        Ok(Self {
            expected_ids_path,
            expected_manifest_path,
            uuid_index_path,
            embeddings_path: embeddings_path
                .ok_or_else(|| "--embeddings is required".to_owned())?,
            output_path,
            retry_ids_output_path,
            expected_dimension,
            expected_model,
            min_coverage_basis_points,
        })
    }
}

fn next_value(args: &mut impl Iterator<Item = String>, option: &str) -> Result<String, String> {
    args.next()
        .ok_or_else(|| format!("{option} requires a value"))
}

fn parse_usize(value: &str, option: &str) -> Result<usize, String> {
    value
        .parse::<usize>()
        .map_err(|error| format!("invalid {option} value {value:?}: {error}"))
}

fn parse_u32(value: &str, option: &str) -> Result<u32, String> {
    value
        .parse::<u32>()
        .map_err(|error| format!("invalid {option} value {value:?}: {error}"))
}

fn usage() -> String {
    "usage: embedding_coverage_check (--expected-ids PATH | --expected-manifest PATH | --uuid-index PATH) --embeddings PATH [--output PATH] [--retry-ids-output PATH] [--expected-dimension N] [--expected-model NAME] [--min-coverage-bps N]".to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_expected_ids_mode() {
        let args = Args::parse([
            "--expected-ids".to_owned(),
            "ids.txt".to_owned(),
            "--embeddings".to_owned(),
            "vectors.jsonl".to_owned(),
            "--expected-dimension".to_owned(),
            "1024".to_owned(),
            "--min-coverage-bps".to_owned(),
            "9950".to_owned(),
        ])
        .unwrap();

        assert_eq!(args.expected_ids_path, Some(PathBuf::from("ids.txt")));
        assert_eq!(args.embeddings_path, PathBuf::from("vectors.jsonl"));
        assert_eq!(args.expected_dimension, Some(1024));
        assert_eq!(args.min_coverage_basis_points, 9_950);
    }

    #[test]
    fn rejects_both_expected_id_sources() {
        assert!(Args::parse([
            "--expected-ids".to_owned(),
            "ids.txt".to_owned(),
            "--uuid-index".to_owned(),
            "uuid.json".to_owned(),
            "--embeddings".to_owned(),
            "vectors.jsonl".to_owned(),
        ])
        .is_err());
    }

    #[test]
    fn parses_retry_ids_output() {
        let args = Args::parse([
            "--expected-ids".to_owned(),
            "ids.txt".to_owned(),
            "--embeddings".to_owned(),
            "vectors.jsonl".to_owned(),
            "--retry-ids-output".to_owned(),
            "retry.txt".to_owned(),
            "--expected-model".to_owned(),
            "bge-m3".to_owned(),
        ])
        .unwrap();

        assert_eq!(args.retry_ids_output_path, Some(PathBuf::from("retry.txt")));
        assert_eq!(args.expected_model, Some("bge-m3".to_owned()));
    }

    #[test]
    fn parses_expected_manifest_mode() {
        let args = Args::parse([
            "--expected-manifest".to_owned(),
            "manifest.jsonl".to_owned(),
            "--embeddings".to_owned(),
            "vectors.jsonl".to_owned(),
        ])
        .unwrap();

        assert_eq!(
            args.expected_manifest_path,
            Some(PathBuf::from("manifest.jsonl"))
        );
    }
}
