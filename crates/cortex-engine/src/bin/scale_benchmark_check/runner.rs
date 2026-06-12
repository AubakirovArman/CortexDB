use std::env;
use std::fs;
use std::time::Instant;

use cortex_engine::{ContextPackOptions, Database, SearchLimit};
use serde_json::json;

use super::args::Args;
use super::metrics::{
    matrix_from_phases, measure_once, measure_repeated, memory_phase, round_ms, sampled_cell_id,
};
use super::workload::{ingest_batches, scale_view};

const VERIFY_AQL: &str =
    r#"VERIFY FACT "scale target onboarding budget approved" IN BRAIN default;"#;
const CONTEXT_AQL: &str = r#"RETRIEVE CONTEXT FOR TASK "onboarding latency budget risk" IN BRAIN default WHERE space = scale AND status = "ready" LIMIT 10 CANDIDATES;"#;

pub(crate) fn run() -> Result<(), String> {
    let args = Args::parse(env::args().skip(1))?;
    fs::remove_dir_all(&args.root).ok();
    fs::create_dir_all(&args.root)
        .map_err(|error| format!("failed to create {}: {error}", args.root.display()))?;

    let started = Instant::now();
    let db_path = args.root.join("db");
    let mut phases = Vec::new();
    let view = scale_view();

    eprintln!("[scale-bench] open_empty");
    let (mut db, phase) = measure_once("open_empty", 1, || Database::open(&db_path))?;
    phases.push(phase);
    eprintln!("[scale-bench] put_batches cells={}", args.cells);
    phases.push(ingest_batches(&mut db, &args)?);
    eprintln!("[scale-bench] memory after_put");
    phases.push(memory_phase("after_put", &db)?);
    eprintln!("[scale-bench] checkpoint");
    phases.push(
        measure_once("checkpoint", args.cells, || {
            db.checkpoint()
                .map(|_| ())
                .map_err(|error| error.to_string())
        })?
        .1,
    );
    eprintln!("[scale-bench] memory after_checkpoint");
    phases.push(memory_phase("after_checkpoint", &db)?);

    if args.samples > 0 {
        eprintln!("[scale-bench] get_latest samples={}", args.samples);
        phases.push(measure_repeated("get_latest", args.samples, |offset| {
            let cell_id = sampled_cell_id(args.cells, offset, args.samples);
            let payload = db
                .get_latest_cell(cell_id)
                .ok_or_else(|| format!("missing sampled cell {}", cell_id.0))?;
            if payload.is_empty() {
                return Err(format!("empty sampled cell {}", cell_id.0));
            }
            Ok(())
        })?);
    }

    if args.search_samples > 0 {
        eprintln!(
            "[scale-bench] keyword_search samples={}",
            args.search_samples
        );
        phases.push(measure_repeated(
            "keyword_search",
            args.search_samples,
            |_| {
                let results = db
                    .search_keyword("onboarding latency budget risk", &view, SearchLimit(10))
                    .map_err(|error| error.to_string())?;
                if results.is_empty() {
                    return Err("keyword search returned no results".to_owned());
                }
                Ok(())
            },
        )?);
    }

    if args.context_samples > 0 {
        eprintln!(
            "[scale-bench] context_pack samples={}",
            args.context_samples
        );
        phases.push(measure_repeated(
            "context_pack",
            args.context_samples,
            |_| {
                let pack = db
                    .context_pack_from_aql(CONTEXT_AQL, &view, ContextPackOptions::default())
                    .map_err(|error| error.to_string())?;
                if pack.cells.is_empty() {
                    return Err("ContextPack returned no cells".to_owned());
                }
                Ok(())
            },
        )?);
    }

    if args.verify_samples > 0 {
        eprintln!("[scale-bench] verify_fact samples={}", args.verify_samples);
        phases.push(measure_repeated(
            "verify_fact",
            args.verify_samples,
            |_| {
                let report = db
                    .verify_fact_aql(VERIFY_AQL, &view)
                    .map_err(|error| error.to_string())?;
                if report.evidence.is_empty() {
                    return Err("VERIFY FACT returned no evidence".to_owned());
                }
                Ok(())
            },
        )?);
    }

    let validation = db.validate_storage_report();
    let mut errors = validation.errors.clone();
    eprintln!("[scale-bench] close");
    phases.push(measure_once("close", 1, || db.close())?.1);
    eprintln!("[scale-bench] restart_open");
    let (reopened, phase) = measure_once("restart_open", args.cells, || Database::open(&db_path))?;
    phases.push(phase);
    let restart_validation = reopened.validate_storage_report();
    errors.extend(
        restart_validation
            .errors
            .iter()
            .map(|error| format!("restart: {error}")),
    );
    reopened
        .close()
        .map_err(|error| format!("restart close failed: {error}"))?;
    write_report(&args, started, &phases, &validation, errors)
}

fn write_report(
    args: &Args,
    started: Instant,
    phases: &[serde_json::Value],
    validation: &cortex_engine::validation::StorageValidationReport,
    errors: Vec<String>,
) -> Result<(), String> {
    let duration_ms = round_ms(started.elapsed().as_secs_f64() * 1000.0);
    let report = json!({
        "schema_version": "cortexdb.scale_benchmark.v1",
        "ok": errors.is_empty(),
        "cells": args.cells,
        "payload_profile": "realistic_0_5kb_to_4kb",
        "samples": {
            "read": args.samples,
            "search": args.search_samples,
            "context": args.context_samples,
            "verify": args.verify_samples,
        },
        "duration_ms": duration_ms,
        "phases": phases,
        "matrix": matrix_from_phases(phases),
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
        return Err(format!("scale benchmark failed: {}", args.report.display()));
    }
    println!("scale benchmark passed: {}", args.report.display());
    Ok(())
}
