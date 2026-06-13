use std::env;
use std::fs;
use std::process::ExitCode;
use std::time::Instant;

use cortex_storage::wal::DurabilityMode;
use serde_json::json;

#[path = "single_node_performance/args.rs"]
mod args;
#[path = "single_node_performance/latency.rs"]
mod latency;
#[path = "single_node_performance/profile.rs"]
mod profile;

use args::Args;
use profile::{collect_profile_errors, round_ms, run_profile};

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
        return args::self_test();
    }

    let root = args.root.clone();
    let report_path = args.report.clone();
    fs::remove_dir_all(&root).ok();
    fs::create_dir_all(&root)
        .map_err(|error| format!("failed to create {}: {error}", root.display()))?;

    let started = Instant::now();
    let strict = run_profile("strict", DurabilityMode::Strict, &args, &root)?;
    let balanced = run_profile("balanced", DurabilityMode::Balanced, &args, &root)?;
    let elapsed_ms = started.elapsed().as_secs_f64() * 1000.0;

    let mut errors = Vec::new();
    if elapsed_ms > args.max_total_ms {
        errors.push(format!(
            "single-node matrix exceeded max-total-ms: {elapsed_ms:.3}"
        ));
    }
    collect_profile_errors(&strict, &mut errors);
    collect_profile_errors(&balanced, &mut errors);
    let report = json!({
        "schema_version": "cortexdb.single_node_performance.v1",
        "ok": errors.is_empty(),
        "cells": args.cells,
        "workload_class": "local_single_node_lifecycle",
        "slo_thresholds": {
            "max_total_ms": args.max_total_ms,
            "min_ingest_cells_per_sec": args.min_ingest_cells_per_sec,
            "max_rss_bytes": args.max_rss_bytes,
        },
        "duration_ms": round_ms(elapsed_ms),
        "profiles": [strict, balanced],
        "errors": errors,
    });
    if let Some(parent) = report_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    }
    fs::write(
        &report_path,
        format!("{}\n", serde_json::to_string_pretty(&report).unwrap()),
    )
    .map_err(|error| format!("failed to write {}: {error}", report_path.display()))?;
    if !report["ok"].as_bool().unwrap_or(false) {
        return Err(format!(
            "single-node performance check failed: {}",
            report_path.display()
        ));
    }
    println!(
        "single-node performance check passed: {}",
        report_path.display()
    );
    Ok(())
}
