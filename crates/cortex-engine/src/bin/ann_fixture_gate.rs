use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use cortex_engine::{evaluate_ann_fixture_baseline, AnnRecallLatencyBaseline};

const DEFAULT_BASELINE_PATH: &str = "crates/cortex-engine/fixtures/ann_fixture_baseline_v1.json";
const DEFAULT_OUTPUT_PATH: &str = "target/ann/ann_fixture_report.json";

#[derive(Debug, PartialEq, Eq)]
struct Args {
    baseline_path: PathBuf,
    output_path: Option<PathBuf>,
}

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
    let args = parse_args(env::args().skip(1))?;
    let bytes = fs::read(&args.baseline_path)
        .map_err(|error| format!("failed to read {}: {error}", args.baseline_path.display()))?;
    let baseline: AnnRecallLatencyBaseline = serde_json::from_slice(&bytes)
        .map_err(|error| format!("failed to parse {}: {error}", args.baseline_path.display()))?;
    let report = evaluate_ann_fixture_baseline(&baseline)
        .map_err(|error| format!("failed to evaluate ANN fixture: {error}"))?;
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
            "ANN fixture gate failed for {}: {}",
            report.baseline_id,
            report.failures.join("; ")
        ))
    }
}

fn parse_args(args: impl IntoIterator<Item = String>) -> Result<Args, String> {
    let mut baseline_path = None;
    let mut output_path = None;
    let mut positional = Vec::new();
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--baseline" => {
                let value = args
                    .next()
                    .ok_or_else(|| "--baseline requires a path".to_owned())?;
                baseline_path = Some(PathBuf::from(value));
            }
            "--output" => {
                let value = args
                    .next()
                    .ok_or_else(|| "--output requires a path".to_owned())?;
                output_path = Some(PathBuf::from(value));
            }
            "--default-output" => {
                output_path = Some(PathBuf::from(DEFAULT_OUTPUT_PATH));
            }
            "--help" | "-h" => {
                return Err(format!(
                    "usage: ann_fixture_gate [--baseline PATH] [--output PATH] [PATH]\n\
                     default baseline: {DEFAULT_BASELINE_PATH}\n\
                     default output: {DEFAULT_OUTPUT_PATH}"
                ));
            }
            value if value.starts_with('-') => {
                return Err(format!("unknown option: {value}"));
            }
            value => positional.push(PathBuf::from(value)),
        }
    }
    if positional.len() > 1 {
        return Err("expected at most one positional baseline path".to_owned());
    }
    if baseline_path.is_some() && !positional.is_empty() {
        return Err("use either --baseline or a positional baseline path, not both".to_owned());
    }
    let baseline_path = baseline_path
        .or_else(|| positional.pop())
        .unwrap_or_else(|| PathBuf::from(DEFAULT_BASELINE_PATH));
    Ok(Args {
        baseline_path,
        output_path,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_positional_baseline() {
        let args = parse_args(["baseline.json".to_owned()]).unwrap();

        assert_eq!(args.baseline_path, PathBuf::from("baseline.json"));
        assert_eq!(args.output_path, None);
    }

    #[test]
    fn parse_named_baseline_and_output() {
        let args = parse_args([
            "--baseline".to_owned(),
            "baseline.json".to_owned(),
            "--output".to_owned(),
            "report.json".to_owned(),
        ])
        .unwrap();

        assert_eq!(args.baseline_path, PathBuf::from("baseline.json"));
        assert_eq!(args.output_path, Some(PathBuf::from("report.json")));
    }

    #[test]
    fn parse_rejects_conflicting_baselines() {
        let error = parse_args([
            "--baseline".to_owned(),
            "baseline.json".to_owned(),
            "other.json".to_owned(),
        ])
        .unwrap_err();

        assert!(error.contains("either --baseline"));
    }
}
