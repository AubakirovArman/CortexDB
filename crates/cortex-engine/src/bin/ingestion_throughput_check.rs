use std::env;
use std::fs;
use std::process::ExitCode;
use std::time::Instant;

use report::{write_markdown, write_report};
use serde_json::{json, Value};
use workload::{run_workload, WorkloadReport};

#[path = "ingestion_throughput_check/args.rs"]
mod args;
#[path = "ingestion_throughput_check/report.rs"]
mod report;
#[path = "ingestion_throughput_check/workload.rs"]
mod workload;

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
    if args.self_test {
        return args::self_test();
    }

    fs::remove_dir_all(&args.root).ok();
    fs::create_dir_all(&args.root)
        .map_err(|error| format!("failed to create {}: {error}", args.root.display()))?;

    let started = Instant::now();
    let workload = run_workload(&args)?;
    let duration_ms = started.elapsed().as_secs_f64() * 1000.0;
    let report = build_report(&args, workload, duration_ms);
    write_report(&args.report, &report)?;
    if let Some(markdown) = &args.markdown {
        write_markdown(markdown, &report)?;
    }
    if report["ok"].as_bool() != Some(true) {
        return Err(format!(
            "ingestion throughput check failed: {}",
            args.report.display()
        ));
    }
    println!(
        "ingestion throughput check passed: {}",
        args.report.display()
    );
    Ok(())
}

fn build_report(args: &Args, workload: WorkloadReport, duration_ms: f64) -> Value {
    let mut errors = workload.errors;
    if workload.throughput_docs_per_sec < args.min_docs_per_sec {
        errors.push(format!(
            "end-to-end docs/sec below threshold: {:.3} < {:.3}",
            workload.throughput_docs_per_sec, args.min_docs_per_sec
        ));
    }
    json!({
        "schema_version": "cortexdb.ingestion_throughput.v1",
        "ok": errors.is_empty(),
        "workload_class": "local_ingestion_embedding_throughput",
        "docs": args.docs,
        "payload_bytes": args.payload_bytes,
        "duration_ms": round_ms(duration_ms),
        "slo_thresholds": {
            "min_docs_per_sec": args.min_docs_per_sec,
        },
        "batching": {
            "ingestion_batch_size": args.ingestion_batch_size,
            "embedding_batch_size": args.embedding_batch_size,
            "ingestion_write_batches": workload.ingestion_write_batches,
            "embedding_batches": workload.embedding_batches,
            "embedding_write_batches": workload.embedding_write_batches,
            "max_embedding_batch_items": workload.max_embedding_batch_items,
        },
        "resume": {
            "resume_after_docs": workload.resume_after_docs,
            "first_run_embedded_items": workload.first_run_embedded_items,
            "first_run_final_debt_items": workload.first_run_final_debt_items,
            "resumed_embedded_items": workload.resumed_embedded_items,
            "final_debt_items": workload.final_debt_items,
            "completed_after_reopen": workload.completed_after_reopen,
        },
        "throughput": {
            "ingest_docs_per_sec": round_ms(workload.ingest_docs_per_sec),
            "embedding_docs_per_sec": round_ms(workload.embedding_docs_per_sec),
            "end_to_end_docs_per_sec": round_ms(workload.throughput_docs_per_sec),
        },
        "quality": {
            "expected_model": workload.expected_model,
            "expected_dimension": workload.expected_dimension,
            "ready_items": workload.ready_items,
            "debt_items": workload.final_debt_items,
            "validation": workload.validation,
        },
        "storage": workload.storage,
        "phases": workload.phases,
        "errors": errors,
    })
}

pub(crate) fn round_ms(value: f64) -> f64 {
    (value * 1000.0).round() / 1000.0
}
