use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use cortex_engine::{evaluate_ann_fixture_baseline, AnnRecallLatencyBaseline};

const DEFAULT_BASELINE_PATH: &str = "crates/cortex-engine/fixtures/ann_fixture_baseline_v1.json";

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
    let baseline_path = env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_BASELINE_PATH));
    let bytes = fs::read(&baseline_path)
        .map_err(|error| format!("failed to read {}: {error}", baseline_path.display()))?;
    let baseline: AnnRecallLatencyBaseline = serde_json::from_slice(&bytes)
        .map_err(|error| format!("failed to parse {}: {error}", baseline_path.display()))?;
    let report = evaluate_ann_fixture_baseline(&baseline)
        .map_err(|error| format!("failed to evaluate ANN fixture: {error}"))?;

    println!("{}", report.as_json());
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
