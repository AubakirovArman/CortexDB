use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use cortex_engine::{evaluate_ann_corpus, parse_ann_metric, AnnCorpusOptions};

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
    let vectors = read_to_string(&args.vectors_path)?;
    let queries = read_to_string(&args.queries_path)?;
    let ground_truth = read_to_string(&args.ground_truth_path)?;
    let report = evaluate_ann_corpus(&vectors, &queries, &ground_truth, args.options)
        .map_err(|error| format!("failed to evaluate ANN corpus: {error}"))?;
    let report_json = report.as_json();
    if let Some(output_path) = args.output_path {
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
        }
        fs::write(&output_path, format!("{report_json}\n"))
            .map_err(|error| format!("failed to write {}: {error}", output_path.display()))?;
    }
    println!("{report_json}");
    if report.passed {
        Ok(())
    } else {
        Err(format!(
            "ANN corpus check failed: {}",
            report.failures.join("; ")
        ))
    }
}

fn read_to_string(path: &PathBuf) -> Result<String, String> {
    fs::read_to_string(path).map_err(|error| format!("failed to read {}: {error}", path.display()))
}

#[derive(Debug, PartialEq, Eq)]
struct Args {
    vectors_path: PathBuf,
    queries_path: PathBuf,
    ground_truth_path: PathBuf,
    output_path: Option<PathBuf>,
    options: AnnCorpusOptions,
}

impl Args {
    fn parse(args: impl IntoIterator<Item = String>) -> Result<Self, String> {
        let mut vectors_path = None;
        let mut queries_path = None;
        let mut ground_truth_path = None;
        let mut output_path = None;
        let mut options = AnnCorpusOptions::default();
        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--vectors" => vectors_path = Some(PathBuf::from(next_value(&mut args, &arg)?)),
                "--queries" => queries_path = Some(PathBuf::from(next_value(&mut args, &arg)?)),
                "--ground-truth" => {
                    ground_truth_path = Some(PathBuf::from(next_value(&mut args, &arg)?))
                }
                "--output" => output_path = Some(PathBuf::from(next_value(&mut args, &arg)?)),
                "--metric" => {
                    options.metric = parse_ann_metric(&next_value(&mut args, &arg)?)
                        .map_err(|error| error.to_string())?;
                }
                "--max-neighbors" => {
                    options.max_neighbors = parse_usize(&next_value(&mut args, &arg)?, &arg)?;
                }
                "--ef-search" => {
                    options.ef_search = parse_usize(&next_value(&mut args, &arg)?, &arg)?;
                }
                "--ef-construction" => {
                    options.ef_construction = parse_usize(&next_value(&mut args, &arg)?, &arg)?;
                }
                "--layer-count" => {
                    options.layer_count = parse_usize(&next_value(&mut args, &arg)?, &arg)?;
                }
                "--min-recall-q16" => {
                    options.min_recall_q16 = parse_u16(&next_value(&mut args, &arg)?, &arg)?;
                }
                "--min-mean-recall-q16" => {
                    options.min_mean_recall_q16 = parse_u16(&next_value(&mut args, &arg)?, &arg)?;
                }
                "--max-p95-latency-nanos" => {
                    options.max_p95_latency_nanos =
                        parse_u128(&next_value(&mut args, &arg)?, &arg)?;
                }
                "--max-max-latency-nanos" => {
                    options.max_max_latency_nanos =
                        parse_u128(&next_value(&mut args, &arg)?, &arg)?;
                }
                "--allow-unsafe" => options.require_production_safe = false,
                "--help" | "-h" => return Err(usage()),
                value => return Err(format!("unknown option: {value}")),
            }
        }
        Ok(Self {
            vectors_path: vectors_path.ok_or_else(|| "--vectors is required".to_owned())?,
            queries_path: queries_path.ok_or_else(|| "--queries is required".to_owned())?,
            ground_truth_path: ground_truth_path
                .ok_or_else(|| "--ground-truth is required".to_owned())?,
            output_path,
            options,
        })
    }
}

fn next_value(args: &mut impl Iterator<Item = String>, option: &str) -> Result<String, String> {
    args.next()
        .ok_or_else(|| format!("{option} requires a value"))
}

fn parse_u16(value: &str, option: &str) -> Result<u16, String> {
    value
        .parse()
        .map_err(|error| format!("{option} must be u16: {error}"))
}

fn parse_usize(value: &str, option: &str) -> Result<usize, String> {
    let parsed = value
        .parse()
        .map_err(|error| format!("{option} must be usize: {error}"))?;
    if parsed == 0 {
        return Err(format!("{option} must be greater than zero"));
    }
    Ok(parsed)
}

fn parse_u128(value: &str, option: &str) -> Result<u128, String> {
    value
        .parse()
        .map_err(|error| format!("{option} must be u128: {error}"))
}

fn usage() -> String {
    "usage: ann_corpus_check --vectors PATH --queries PATH --ground-truth PATH \
     [--metric dot_product|cosine|l2] [--max-neighbors N] [--ef-search N] \
     [--ef-construction N] [--layer-count N] [--output PATH]"
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use cortex_engine::DistanceMetric;

    #[test]
    fn parse_required_paths() {
        let args = Args::parse([
            "--vectors".to_owned(),
            "vectors.jsonl".to_owned(),
            "--queries".to_owned(),
            "queries.jsonl".to_owned(),
            "--ground-truth".to_owned(),
            "truth.jsonl".to_owned(),
        ])
        .unwrap();

        assert_eq!(args.vectors_path, PathBuf::from("vectors.jsonl"));
        assert_eq!(args.queries_path, PathBuf::from("queries.jsonl"));
        assert_eq!(args.ground_truth_path, PathBuf::from("truth.jsonl"));
    }

    #[test]
    fn parse_metric_and_thresholds() {
        let args = Args::parse([
            "--vectors".to_owned(),
            "v".to_owned(),
            "--queries".to_owned(),
            "q".to_owned(),
            "--ground-truth".to_owned(),
            "g".to_owned(),
            "--metric".to_owned(),
            "l2".to_owned(),
            "--min-recall-q16".to_owned(),
            "50000".to_owned(),
            "--max-neighbors".to_owned(),
            "16".to_owned(),
            "--ef-search".to_owned(),
            "128".to_owned(),
            "--ef-construction".to_owned(),
            "256".to_owned(),
            "--layer-count".to_owned(),
            "5".to_owned(),
        ])
        .unwrap();

        assert_eq!(args.options.metric, DistanceMetric::L2);
        assert_eq!(args.options.min_recall_q16, 50_000);
        assert_eq!(args.options.max_neighbors, 16);
        assert_eq!(args.options.ef_search, 128);
        assert_eq!(args.options.ef_construction, 256);
        assert_eq!(args.options.layer_count, 5);
    }

    #[test]
    fn parse_rejects_zero_tuning_knob() {
        let error = Args::parse([
            "--vectors".to_owned(),
            "v".to_owned(),
            "--queries".to_owned(),
            "q".to_owned(),
            "--ground-truth".to_owned(),
            "g".to_owned(),
            "--ef-search".to_owned(),
            "0".to_owned(),
        ])
        .unwrap_err();

        assert!(error.contains("greater than zero"));
    }
}
