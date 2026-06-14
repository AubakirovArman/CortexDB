use std::env;
use std::fs;
use std::process::ExitCode;
use std::time::Instant;

#[path = "concurrent_read_benchmark_check/args.rs"]
mod args;
#[path = "concurrent_read_benchmark_check/report.rs"]
mod report;
#[path = "concurrent_read_benchmark_check/support.rs"]
mod support;
#[path = "concurrent_read_benchmark_check/workload.rs"]
mod workload;

use args::Args;
use report::{collect_slo_errors, write_json_report, write_markdown_report};
use workload::{run_mode, Mode};

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
    if args.self_test {
        args::self_test()?;
        report::self_test()?;
        println!("concurrent read benchmark self-test passed");
        return Ok(());
    }

    fs::remove_dir_all(&args.root).ok();
    fs::create_dir_all(&args.root)
        .map_err(|error| format!("failed to create {}: {error}", args.root.display()))?;

    let started = Instant::now();
    let modes = vec![
        run_mode(Mode::ActorRouteShared, &args, &args.root)?,
        run_mode(Mode::RwLockDirect, &args, &args.root)?,
    ];

    let errors = collect_slo_errors(&args, &modes);
    let duration_ms = report::round_ms(started.elapsed().as_secs_f64() * 1000.0);
    let value = report::build_report(&args, duration_ms, modes, errors.clone());
    write_json_report(&args.report, &value)?;
    write_markdown_report(&args.markdown, &value)?;

    if !errors.is_empty() {
        return Err(format!(
            "concurrent read benchmark failed: {}",
            args.report.display()
        ));
    }

    println!(
        "concurrent read benchmark passed: {}",
        args.report.display()
    );
    Ok(())
}
