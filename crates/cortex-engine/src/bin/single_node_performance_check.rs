use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::process::ExitCode;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use cortex_aql::{AgentId, AgentView, BrainId, MemoryType, RetrievalMode, ScopeId, Q16_ZERO};
use cortex_core::CellId;
use cortex_engine::{scope_id, ContextPackOptions, Database, DatabaseOptions, SearchLimit};
use cortex_storage::wal::DurabilityMode;
use serde_json::{json, Value};

#[path = "single_node_performance/args.rs"]
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
    if args.self_test {
        return args::self_test();
    }

    let root = args.root;
    let report_path = args.report;
    fs::remove_dir_all(&root).ok();
    fs::create_dir_all(&root)
        .map_err(|error| format!("failed to create {}: {error}", root.display()))?;

    let started = Instant::now();
    let strict = run_profile("strict", DurabilityMode::Strict, args.cells, &root)?;
    let balanced = run_profile("balanced", DurabilityMode::Balanced, args.cells, &root)?;
    let elapsed_ms = started.elapsed().as_secs_f64() * 1000.0;

    let mut errors = Vec::new();
    if elapsed_ms > args.max_total_ms {
        errors.push(format!(
            "single-node matrix exceeded max-total-ms: {elapsed_ms:.3}"
        ));
    }
    let report = json!({
        "schema_version": "cortexdb.single_node_performance.v1",
        "ok": errors.is_empty(),
        "cells": args.cells,
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

fn run_profile(
    label: &str,
    durability_mode: DurabilityMode,
    cells: usize,
    root: &std::path::Path,
) -> Result<Value, String> {
    let db_path = root.join(format!("{label}-{}", unique_id()));
    let options = DatabaseOptions {
        durability_mode,
        ..DatabaseOptions::default()
    };
    let mut phases = Vec::new();
    let view = perf_view();
    let query = r#"RETRIEVE CONTEXT FOR TASK "performance budget" IN BRAIN default WHERE space = perf AND status = "ready" LIMIT 10 CANDIDATES;"#;

    let (mut db, phase) = measure("open_empty", 1, || {
        Database::open_with_options(&db_path, options)
    })?;
    phases.push(phase);

    let payloads = (1..=cells)
        .map(|index| (CellId(index as u64), payload(index)))
        .collect::<Vec<_>>();
    let (_, phase) = measure("put_batch", cells, || db.put_cells(payloads))?;
    phases.push(phase);

    let (_, phase) = measure("get_latest", cells, || {
        for index in 1..=cells {
            let payload = db
                .get_latest_cell(CellId(index as u64))
                .ok_or_else(|| format!("missing cell after put: {index}"))?;
            if payload.is_empty() {
                return Err(format!("empty payload after put: {index}"));
            }
        }
        Ok(())
    })?;
    phases.push(phase);

    let (_, phase) = measure("keyword_search", 25, || {
        for _ in 0..25 {
            let results = db
                .search_keyword("budget ready", &view, SearchLimit(10))
                .map_err(|error| error.to_string())?;
            if results.is_empty() {
                return Err("keyword search returned no results".to_owned());
            }
        }
        Ok(())
    })?;
    phases.push(phase);

    let (_, phase) = measure("context_pack", 10, || {
        for _ in 0..10 {
            let pack = db
                .context_pack_from_aql(query, &view, ContextPackOptions::default())
                .map_err(|error| error.to_string())?;
            if pack.cells.is_empty() {
                return Err("ContextPack returned no cells".to_owned());
            }
        }
        Ok(())
    })?;
    phases.push(phase);

    let (_, phase) = measure("checkpoint", cells, || db.checkpoint())?;
    phases.push(phase);

    let (_, phase) = measure("compact", cells, || db.compact())?;
    phases.push(phase);

    let (_, phase) = measure("close", 1, || db.close())?;
    phases.push(phase);

    let (db, phase) = measure("restart_open", cells, || {
        Database::open_with_options(&db_path, options)
    })?;
    phases.push(phase);

    let validation = db.validate_storage_report();
    if !validation.errors.is_empty() {
        return Err(format!(
            "{label} validation failed: {}",
            validation.errors.join("; ")
        ));
    }
    let stats = db
        .storage_stats()
        .map_err(|error| format!("{label} stats failed: {error}"))?;
    db.close()
        .map_err(|error| format!("{label} final close failed: {error}"))?;

    Ok(json!({
        "name": label,
        "durability_mode": format!("{durability_mode:?}").to_lowercase(),
        "phases": phases,
        "validation": {
            "manifest_ok": validation.manifest_ok,
            "wal_ok": validation.wal_ok,
            "live_segments_checked": validation.live_segments_checked,
            "cells_checked": validation.cells_checked,
        },
        "stats": {
            "current_seq": stats.current_seq.0,
            "checkpoint_seq": stats.checkpoint_seq.0,
            "live_segments": stats.live_segments,
            "wal_size_bytes": stats.wal_size_bytes,
        }
    }))
}

fn measure<T, E, F>(name: &str, units: usize, call: F) -> Result<(T, Value), String>
where
    E: std::fmt::Display,
    F: FnOnce() -> Result<T, E>,
{
    let start = Instant::now();
    let value = call().map_err(|error| error.to_string())?;
    let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;
    let throughput = if elapsed_ms > 0.0 {
        (units as f64) / (elapsed_ms / 1000.0)
    } else {
        0.0
    };
    Ok((
        value,
        json!({
            "name": name,
            "units": units,
            "elapsed_ms": round_ms(elapsed_ms),
            "throughput_per_sec": round_ms(throughput),
        }),
    ))
}

fn payload(index: usize) -> Vec<u8> {
    format!(
        "scope=perf\nstatus=ready\ntype=fact\nsource=single-node-{index}\n\nperformance budget ready cell {index}"
    )
    .into_bytes()
}

fn perf_view() -> AgentView {
    AgentView {
        agent_id: AgentId(1),
        label: Some("single-node-performance".to_owned()),
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

fn round_ms(value: f64) -> f64 {
    (value * 1000.0).round() / 1000.0
}

fn unique_id() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0)
}
