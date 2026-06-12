use std::process::ExitCode;

#[path = "scale_benchmark_check/args.rs"]
mod args;
#[path = "scale_benchmark_check/metrics.rs"]
mod metrics;
#[path = "scale_benchmark_check/runner.rs"]
mod runner;
#[path = "scale_benchmark_check/workload.rs"]
mod workload;

fn main() -> ExitCode {
    match runner::run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
