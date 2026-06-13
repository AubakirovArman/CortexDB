use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::process::ExitCode;
use std::time::Instant;

use cortex_aql::{AgentId, AgentView, BrainId, MemoryType, RetrievalMode, ScopeId, Q16_ZERO};
use cortex_core::CellId;
use cortex_engine::{scope_id, Database};
use serde_json::{json, Value};

const FACT: &str = "verifier target alpha budget approved";
const VERIFY_AQL: &str = r#"VERIFY FACT "verifier target alpha budget approved" IN BRAIN default;"#;

#[path = "verify_performance_check/args.rs"]
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
    fs::remove_dir_all(&args.root).ok();
    fs::create_dir_all(&args.root)
        .map_err(|error| format!("failed to create {}: {error}", args.root.display()))?;

    let started = Instant::now();
    let mut phases = Vec::new();
    let view = perf_view();
    let db_path = args.root.join("db");
    let mut db = Database::open(&db_path).map_err(|error| error.to_string())?;

    let cells = build_cells(args.cells);
    phases.push(measure("put_cells", args.cells, || {
        db.put_cells(cells)
            .map(|_| ())
            .map_err(|error| error.to_string())
    })?);
    phases.push(measure("checkpoint", args.cells, || {
        db.checkpoint()
            .map(|_| ())
            .map_err(|error| error.to_string())
    })?);

    let mut warmup_latencies = Vec::with_capacity(args.warmup_samples);
    for _ in 0..args.warmup_samples {
        let sample_started = Instant::now();
        verify_target(&db, &view)?;
        warmup_latencies.push(sample_started.elapsed().as_secs_f64() * 1000.0);
    }
    phases.push(json!({
        "name": "verify_fact_warmup",
        "samples": args.warmup_samples,
        "latency": latency_summary(&warmup_latencies),
    }));

    let mut latencies = Vec::with_capacity(args.samples);
    for _ in 0..args.samples {
        let sample_started = Instant::now();
        verify_target(&db, &view)?;
        latencies.push(sample_started.elapsed().as_secs_f64() * 1000.0);
    }
    let verify_latency = latency_summary(&latencies);
    let p95_ms = verify_latency["p95_ms"].as_f64().unwrap_or(f64::MAX);
    let mut errors = Vec::new();
    if p95_ms > args.max_p95_ms {
        errors.push(format!(
            "verify_fact p95 exceeded threshold: {p95_ms:.3} > {:.3}",
            args.max_p95_ms
        ));
    }
    phases.push(json!({
        "name": "verify_fact",
        "samples": args.samples,
        "latency": verify_latency,
    }));

    let elapsed_ms = started.elapsed().as_secs_f64() * 1000.0;
    let validation = db.validate_storage_report();
    if !validation.errors.is_empty() {
        errors.push(format!(
            "storage validation failed: {}",
            validation.errors.join("; ")
        ));
    }

    let report = json!({
        "schema_version": "cortexdb.verify_performance.v1",
        "ok": errors.is_empty(),
        "cells": args.cells,
        "warmup_samples": args.warmup_samples,
        "samples": args.samples,
        "workload_class": "checkpointed_verify_fact_rare_term",
        "query": VERIFY_AQL,
        "slo_thresholds": {
            "max_p95_ms": args.max_p95_ms,
        },
        "duration_ms": round_ms(elapsed_ms),
        "phases": phases,
        "validation": {
            "manifest_ok": validation.manifest_ok,
            "wal_ok": validation.wal_ok,
            "live_segments_checked": validation.live_segments_checked,
            "cells_checked": validation.cells_checked,
        },
        "errors": errors,
    });
    if let Some(parent) = args.report.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    }
    fs::write(
        &args.report,
        format!("{}\n", serde_json::to_string_pretty(&report).unwrap()),
    )
    .map_err(|error| format!("failed to write {}: {error}", args.report.display()))?;
    if !report["ok"].as_bool().unwrap_or(false) {
        return Err(format!(
            "verify performance check failed: {}",
            args.report.display()
        ));
    }
    println!("verify performance check passed: {}", args.report.display());
    Ok(())
}

fn verify_target(db: &Database, view: &AgentView) -> Result<(), String> {
    let report = db
        .verify_fact_aql(VERIFY_AQL, view)
        .map_err(|error| error.to_string())?;
    if report.evidence.first().map(|item| item.cell_id) != Some(CellId(1)) {
        return Err("verify_fact did not return the target evidence cell".to_owned());
    }
    Ok(())
}

fn build_cells(cells: usize) -> Vec<(CellId, Vec<u8>)> {
    let mut payloads = Vec::with_capacity(cells);
    payloads.push((CellId(1), target_payload()));
    for index in 2..=cells {
        payloads.push((CellId(index as u64), noise_payload(index)));
    }
    payloads
}

fn target_payload() -> Vec<u8> {
    format!("scope=perf\nstatus=ready\ntype=fact\nsource=verify-target\n\n{FACT}").into_bytes()
}

fn noise_payload(index: usize) -> Vec<u8> {
    format!(
        "scope=perf\nstatus=ready\ntype=fact\nsource=verify-noise-{index}\n\nnoise corpus cell {index} unrelated operations note"
    )
    .into_bytes()
}

fn perf_view() -> AgentView {
    AgentView {
        agent_id: AgentId(1),
        label: Some("verify-performance".to_owned()),
        readable_brains: BTreeSet::from([BrainId(1)]),
        readable_scopes: BTreeSet::from([scope_id("perf")]),
        writable_scopes: BTreeSet::from([scope_id("perf")]),
        allowed_modes: BTreeSet::from([RetrievalMode::Fast, RetrievalMode::Balanced]),
        allowed_memory_types: BTreeSet::from([
            MemoryType::Decision,
            MemoryType::Preference,
            MemoryType::WorkflowResult,
            MemoryType::ErrorLog,
            MemoryType::Observation,
        ]),
        max_context_budget_tokens: 10_000,
        default_context_budget_tokens: 1_000,
        max_candidate_limit: 1_000,
        default_candidate_limit: 10,
        min_required_confidence_q16: Q16_ZERO,
        max_ttl_seconds: None,
        allow_remember: true,
        allow_verify_fact: true,
        allow_audit_mode: false,
        require_citations_by_default: false,
        private_scope: Some(ScopeId(999)),
    }
}

fn measure<E, F>(name: &str, units: usize, call: F) -> Result<Value, String>
where
    E: std::fmt::Display,
    F: FnOnce() -> Result<(), E>,
{
    let started = Instant::now();
    call().map_err(|error| error.to_string())?;
    let elapsed_ms = started.elapsed().as_secs_f64() * 1000.0;
    let throughput = if elapsed_ms > 0.0 {
        (units as f64) / (elapsed_ms / 1000.0)
    } else {
        0.0
    };
    Ok(json!({
        "name": name,
        "units": units,
        "elapsed_ms": round_ms(elapsed_ms),
        "throughput_per_sec": round_ms(throughput),
    }))
}

fn latency_summary(values: &[f64]) -> Value {
    json!({
        "count": values.len(),
        "p50_ms": round_ms(percentile(values, 0.50)),
        "p95_ms": round_ms(percentile(values, 0.95)),
        "p99_ms": round_ms(percentile(values, 0.99)),
        "max_ms": round_ms(values.iter().copied().fold(0.0, f64::max)),
    })
}

fn percentile(values: &[f64], percent: f64) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let mut ordered = values.to_vec();
    ordered.sort_by(|left, right| left.total_cmp(right));
    let index = ((ordered.len() - 1) as f64 * percent).floor() as usize;
    ordered[index.min(ordered.len() - 1)]
}

fn round_ms(value: f64) -> f64 {
    (value * 1000.0).round() / 1000.0
}
