use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use cortex_engine::{evaluate_ann_metric_matrix, AnnMetricMatrixBaseline};

const DEFAULT_BASELINE_PATH: &str =
    "crates/cortex-engine/fixtures/ann_metric_matrix_baseline_v1.json";
const DEFAULT_OUTPUT_PATH: &str = "target/ann/ann_metric_matrix_report.json";

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
    let baseline_bytes = fs::read(&args.baseline_path)
        .map_err(|error| format!("failed to read {}: {error}", args.baseline_path.display()))?;
    let baseline: AnnMetricMatrixBaseline = serde_json::from_slice(&baseline_bytes)
        .map_err(|error| format!("failed to parse {}: {error}", args.baseline_path.display()))?;
    let fixture_text = fs::read_to_string(&baseline.fixture_path)
        .map_err(|error| format!("failed to read {}: {error}", baseline.fixture_path))?;
    let report = evaluate_ann_metric_matrix(&baseline, &fixture_text)
        .map_err(|error| format!("failed to evaluate ANN metric matrix: {error}"))?;
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
            "ANN metric matrix failed for {}: {}",
            report.baseline_id,
            report.failures.join("; ")
        ))
    }
}

#[derive(Debug, PartialEq, Eq)]
struct Args {
    baseline_path: PathBuf,
    output_path: Option<PathBuf>,
}

impl Args {
    fn parse(args: impl IntoIterator<Item = String>) -> Result<Self, String> {
        let mut baseline_path = None;
        let mut output_path = None;
        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--baseline" => baseline_path = Some(PathBuf::from(next_value(&mut args, &arg)?)),
                "--output" => output_path = Some(PathBuf::from(next_value(&mut args, &arg)?)),
                "--default-output" => output_path = Some(PathBuf::from(DEFAULT_OUTPUT_PATH)),
                "--help" | "-h" => {
                    return Err(format!(
                        "usage: ann_metric_matrix_check [--baseline PATH] [--output PATH]\n\
                         default baseline: {DEFAULT_BASELINE_PATH}\n\
                         default output: {DEFAULT_OUTPUT_PATH}"
                    ));
                }
                value => return Err(format!("unknown option: {value}")),
            }
        }
        Ok(Self {
            baseline_path: baseline_path.unwrap_or_else(|| PathBuf::from(DEFAULT_BASELINE_PATH)),
            output_path,
        })
    }
}

fn next_value(args: &mut impl Iterator<Item = String>, option: &str) -> Result<String, String> {
    args.next()
        .ok_or_else(|| format!("{option} requires a path"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_defaults() {
        let args = Args::parse([]).unwrap();

        assert_eq!(args.baseline_path, PathBuf::from(DEFAULT_BASELINE_PATH));
        assert_eq!(args.output_path, None);
    }

    #[test]
    fn parse_named_paths() {
        let args = Args::parse([
            "--baseline".to_owned(),
            "baseline.json".to_owned(),
            "--output".to_owned(),
            "report.json".to_owned(),
        ])
        .unwrap();

        assert_eq!(args.baseline_path, PathBuf::from("baseline.json"));
        assert_eq!(args.output_path, Some(PathBuf::from("report.json")));
    }
}
