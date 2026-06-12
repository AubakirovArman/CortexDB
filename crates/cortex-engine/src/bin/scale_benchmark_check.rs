use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Instant;

use cortex_aql::{AgentId, AgentView, BrainId, MemoryType, RetrievalMode, ScopeId, Q16_ZERO};
use cortex_core::CellId;
use cortex_engine::{scope_id, ContextPackOptions, Database, SearchLimit};
use serde_json::{json, Value};

const VERIFY_AQL: &str =
    r#"VERIFY FACT "scale target onboarding budget approved" IN BRAIN default;"#;
const CONTEXT_AQL: &str = r#"RETRIEVE CONTEXT FOR TASK "onboarding latency budget risk" IN BRAIN default WHERE space = scale AND status = "ready" LIMIT 10 CANDIDATES;"#;

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
        "matrix": matrix_from_phases(&phases),
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

fn ingest_batches(db: &mut Database, args: &Args) -> Result<Value, String> {
    let started = Instant::now();
    let mut next = 1usize;
    while next <= args.cells {
        let end = next
            .saturating_add(args.batch_size)
            .saturating_sub(1)
            .min(args.cells);
        let batch = (next..=end)
            .map(|index| (CellId(index as u64), payload(index)))
            .collect::<Vec<_>>();
        db.put_cells(batch).map_err(|error| error.to_string())?;
        next = end + 1;
    }
    let elapsed_ms = started.elapsed().as_secs_f64() * 1000.0;
    Ok(json!({
        "name": "put_batches",
        "units": args.cells,
        "batch_size": args.batch_size,
        "elapsed_ms": round_ms(elapsed_ms),
        "throughput_per_sec": throughput(args.cells, elapsed_ms),
    }))
}

fn payload(index: usize) -> Vec<u8> {
    let scope = match index % 5 {
        0 => "scale",
        1 => "scale",
        2 => "scale:team-a",
        3 => "scale:team-b",
        _ => "scale:archive",
    };
    let topic = match index % 7 {
        0 => "onboarding latency budget risk",
        1 => "checkpoint storage wal recovery",
        2 => "context pack retrieval evidence",
        3 => "agent memory verification source",
        4 => "search lexical semantic hybrid",
        5 => "tenant scope permissions audit",
        _ => "replication repair manifest segment",
    };
    let target = if index == 1 {
        "scale target onboarding budget approved"
    } else {
        "scale benchmark background evidence"
    };
    let target_len = 512 + ((index.wrapping_mul(7919)) % 3585);
    let mut text = format!(
        "scope={scope}\nstatus=ready\ntype=fact\nsource=scale-doc-{index}\ncreated={}\n\n{target}. {topic}. ",
        1_700_000_000u64 + index as u64
    );
    while text.len() < target_len {
        text.push_str(topic);
        text.push_str(". operational note with owner, date, risk, budget, status, and evidence. ");
    }
    text.truncate(target_len);
    text.into_bytes()
}

fn memory_phase(name: &str, db: &Database) -> Result<Value, String> {
    let stats = db.storage_stats().map_err(|error| error.to_string())?;
    let (rss_bytes, peak_rss_bytes) = linux_proc_status_memory_bytes().unwrap_or((0, 0));
    Ok(json!({
        "name": name,
        "resource_usage": {
            "rss_bytes": rss_bytes,
            "peak_rss_bytes": peak_rss_bytes,
        },
        "storage_estimates": {
            "memtable_payload_bytes": stats.memtable_payload_bytes,
            "estimated_memtable_bytes": stats.estimated_memtable_bytes,
            "estimated_index_bytes": stats.estimated_index_bytes,
            "estimated_context_pack_bytes": stats.estimated_context_pack_bytes,
            "estimated_total_memory_bytes": stats.estimated_total_memory_bytes,
            "live_segment_bytes": stats.live_segment_bytes,
            "logical_payload_bytes": stats.logical_payload_bytes,
        }
    }))
}

fn measure_once<T, E, F>(name: &str, units: usize, call: F) -> Result<(T, Value), String>
where
    E: std::fmt::Display,
    F: FnOnce() -> Result<T, E>,
{
    let started = Instant::now();
    let value = call().map_err(|error| error.to_string())?;
    let elapsed_ms = started.elapsed().as_secs_f64() * 1000.0;
    Ok((
        value,
        json!({
            "name": name,
            "units": units,
            "elapsed_ms": round_ms(elapsed_ms),
            "throughput_per_sec": throughput(units, elapsed_ms),
        }),
    ))
}

fn measure_repeated<E, F>(name: &str, units: usize, mut call: F) -> Result<Value, String>
where
    E: std::fmt::Display,
    F: FnMut(usize) -> Result<(), E>,
{
    let started = Instant::now();
    let mut latencies = Vec::with_capacity(units);
    for offset in 0..units {
        let item_started = Instant::now();
        call(offset).map_err(|error| error.to_string())?;
        latencies.push(item_started.elapsed().as_secs_f64() * 1000.0);
    }
    let elapsed_ms = started.elapsed().as_secs_f64() * 1000.0;
    Ok(json!({
        "name": name,
        "units": units,
        "elapsed_ms": round_ms(elapsed_ms),
        "throughput_per_sec": throughput(units, elapsed_ms),
        "latency": latency_summary(&latencies),
    }))
}

fn matrix_from_phases(phases: &[Value]) -> Value {
    let mut matrix = serde_json::Map::new();
    for phase in phases {
        let Some(name) = phase["name"].as_str() else {
            continue;
        };
        let entry = if let Some(latency) = phase.get("latency") {
            json!({
                "p50_ms": latency["p50_ms"],
                "p95_ms": latency["p95_ms"],
                "p99_ms": latency["p99_ms"],
                "max_ms": latency["max_ms"],
            })
        } else if phase.get("elapsed_ms").is_some() {
            json!({
                "elapsed_ms": phase["elapsed_ms"],
                "throughput_per_sec": phase["throughput_per_sec"],
            })
        } else if phase.get("resource_usage").is_some() {
            json!({
                "rss_bytes": phase["resource_usage"]["rss_bytes"],
                "peak_rss_bytes": phase["resource_usage"]["peak_rss_bytes"],
                "estimated_total_memory_bytes": phase["storage_estimates"]["estimated_total_memory_bytes"],
            })
        } else {
            continue;
        };
        matrix.insert(name.to_owned(), entry);
    }
    Value::Object(matrix)
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

fn sampled_cell_id(cells: usize, offset: usize, samples: usize) -> CellId {
    let stride = cells.max(1) / samples.max(1);
    let index = 1 + offset.saturating_mul(stride.max(1)) % cells.max(1);
    CellId(index as u64)
}

fn throughput(units: usize, elapsed_ms: f64) -> f64 {
    if elapsed_ms <= 0.0 {
        return 0.0;
    }
    round_ms((units as f64) / (elapsed_ms / 1000.0))
}

fn round_ms(value: f64) -> f64 {
    (value * 1000.0).round() / 1000.0
}

fn scale_view() -> AgentView {
    AgentView {
        agent_id: AgentId(1),
        label: Some("scale-benchmark".to_owned()),
        readable_brains: BTreeSet::from([BrainId(1)]),
        readable_scopes: BTreeSet::from([
            scope_id("scale"),
            scope_id("scale:team-a"),
            scope_id("scale:team-b"),
            scope_id("scale:archive"),
        ]),
        writable_scopes: BTreeSet::from([scope_id("scale")]),
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

fn linux_proc_status_memory_bytes() -> Option<(u64, u64)> {
    let status = fs::read_to_string("/proc/self/status").ok()?;
    let mut rss_bytes = 0;
    let mut peak_rss_bytes = 0;
    for line in status.lines() {
        if let Some(value) = line.strip_prefix("VmRSS:") {
            rss_bytes = parse_status_kib(value).unwrap_or(0);
        } else if let Some(value) = line.strip_prefix("VmHWM:") {
            peak_rss_bytes = parse_status_kib(value).unwrap_or(0);
        }
    }
    Some((rss_bytes, peak_rss_bytes.max(rss_bytes)))
}

fn parse_status_kib(value: &str) -> Option<u64> {
    let kib = value.split_whitespace().next()?.parse::<u64>().ok()?;
    kib.checked_mul(1024)
}

struct Args {
    root: PathBuf,
    report: PathBuf,
    cells: usize,
    samples: usize,
    search_samples: usize,
    context_samples: usize,
    verify_samples: usize,
    batch_size: usize,
}

impl Args {
    fn parse(values: impl Iterator<Item = String>) -> Result<Self, String> {
        let mut args = Self {
            root: PathBuf::from("target/scale-bench"),
            report: PathBuf::from("target/scale-bench/report.json"),
            cells: 100_000,
            samples: 100,
            search_samples: 100,
            context_samples: 10,
            verify_samples: 10,
            batch_size: 5_000,
        };
        let mut values = values.peekable();
        while let Some(arg) = values.next() {
            match arg.as_str() {
                "--root" => args.root = PathBuf::from(next_value(&mut values, "--root")?),
                "--report" => args.report = PathBuf::from(next_value(&mut values, "--report")?),
                "--cells" => {
                    args.cells = parse_usize(next_value(&mut values, "--cells")?, "--cells")?
                }
                "--samples" => {
                    args.samples = parse_usize(next_value(&mut values, "--samples")?, "--samples")?
                }
                "--search-samples" => {
                    args.search_samples = parse_usize(
                        next_value(&mut values, "--search-samples")?,
                        "--search-samples",
                    )?
                }
                "--context-samples" => {
                    args.context_samples = parse_usize(
                        next_value(&mut values, "--context-samples")?,
                        "--context-samples",
                    )?
                }
                "--verify-samples" => {
                    args.verify_samples = parse_usize(
                        next_value(&mut values, "--verify-samples")?,
                        "--verify-samples",
                    )?
                }
                "--batch-size" => {
                    args.batch_size =
                        parse_usize(next_value(&mut values, "--batch-size")?, "--batch-size")?
                }
                "--help" | "-h" => return Err(help_text()),
                unknown => return Err(format!("unknown argument: {unknown}\n{}", help_text())),
            }
        }
        for (name, value) in [("--cells", args.cells), ("--batch-size", args.batch_size)] {
            if value == 0 {
                return Err(format!("{name} must be positive"));
            }
        }
        Ok(args)
    }
}

fn next_value(
    values: &mut std::iter::Peekable<impl Iterator<Item = String>>,
    flag: &str,
) -> Result<String, String> {
    values
        .next()
        .ok_or_else(|| format!("missing value for {flag}"))
}

fn parse_usize(value: String, flag: &str) -> Result<usize, String> {
    value
        .parse::<usize>()
        .map_err(|_| format!("invalid value for {flag}: {value}"))
}

fn help_text() -> String {
    "usage: scale_benchmark_check [--root PATH] [--report PATH] [--cells N] [--samples N] [--search-samples N] [--context-samples N] [--verify-samples N] [--batch-size N]".to_owned()
}
