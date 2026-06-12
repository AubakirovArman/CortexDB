use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Instant;

use cortex_core::CellId;
use cortex_engine::Database;
use serde_json::{json, Value};

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

    let clone_gate = payload_clone_gate_report();
    let started = Instant::now();
    let mut samples = Vec::new();
    samples.push(memory_sample("process_start"));

    let db_path = args.root.join("db");
    let mut db = Database::open(&db_path).map_err(|error| error.to_string())?;
    samples.push(memory_sample("open_empty"));

    db.put_cells(build_cells(args.cells))
        .map_err(|error| error.to_string())?;
    let after_put_stats = db.storage_stats().map_err(|error| error.to_string())?;
    samples.push(memory_sample("after_put"));

    db.checkpoint().map_err(|error| error.to_string())?;
    let after_checkpoint_stats = db.storage_stats().map_err(|error| error.to_string())?;
    samples.push(memory_sample("after_checkpoint"));

    let validation = db.validate_storage_report();
    if !validation.errors.is_empty() {
        return Err(format!(
            "storage validation failed: {}",
            validation.errors.join("; ")
        ));
    }

    let final_sample = memory_sample("final");
    let mut errors = clone_gate_errors(&clone_gate);
    if let Some(error) = estimate_ratio_error(&final_sample, &after_checkpoint_stats, &args) {
        errors.push(error);
    }

    let report = json!({
        "schema_version": "cortexdb.memory_profile.v1",
        "ok": errors.is_empty(),
        "cells": args.cells,
        "duration_ms": round_ms(started.elapsed().as_secs_f64() * 1000.0),
        "resource_samples": samples,
        "final_resource_usage": final_sample,
        "storage_estimates": {
            "after_put": storage_estimate_report(&after_put_stats),
            "after_checkpoint": storage_estimate_report(&after_checkpoint_stats),
        },
        "estimate_vs_rss": estimate_vs_rss_report(&final_sample, &after_checkpoint_stats),
        "payload_clone_gate": clone_gate,
        "allocation_observers": {
            "dhat": {
                "available": false,
                "reason": "not enabled; no new profiling dependency is added by default"
            },
            "jemalloc": {
                "available": false,
                "reason": "not linked; RSS and storage estimates are the default portable profile"
            }
        },
        "slo_thresholds": {
            "max_rss_to_estimated_total_ratio": args.max_rss_to_estimated_total_ratio,
        },
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
        return Err(format!("memory profile failed: {}", args.report.display()));
    }
    println!("memory profile passed: {}", args.report.display());
    Ok(())
}

fn build_cells(cells: usize) -> Vec<(CellId, Vec<u8>)> {
    (1..=cells)
        .map(|index| {
            (
                CellId(index as u64),
                format!(
                    "scope=memory:profile\nstatus=ready\ntype=fact\nsource=memory-profile-{index}\n\nmemory profile payload {index} alpha beta gamma"
                )
                .into_bytes(),
            )
        })
        .collect()
}

fn memory_sample(label: &str) -> Value {
    let (rss_bytes, peak_rss_bytes) = linux_proc_status_memory_bytes().unwrap_or((0, 0));
    json!({
        "label": label,
        "rss_bytes": rss_bytes,
        "peak_rss_bytes": peak_rss_bytes,
    })
}

fn storage_estimate_report(stats: &cortex_engine::StorageStats) -> Value {
    json!({
        "memtable_payload_bytes": stats.memtable_payload_bytes,
        "estimated_memtable_bytes": stats.estimated_memtable_bytes,
        "estimated_index_bytes": stats.estimated_index_bytes,
        "estimated_context_pack_bytes": stats.estimated_context_pack_bytes,
        "estimated_total_memory_bytes": stats.estimated_total_memory_bytes,
        "live_segment_bytes": stats.live_segment_bytes,
        "logical_payload_bytes": stats.logical_payload_bytes,
    })
}

fn estimate_vs_rss_report(sample: &Value, stats: &cortex_engine::StorageStats) -> Value {
    let rss = sample["rss_bytes"].as_u64().unwrap_or(0);
    let peak = sample["peak_rss_bytes"].as_u64().unwrap_or(0);
    let estimated = stats.estimated_total_memory_bytes as u64;
    json!({
        "rss_bytes": rss,
        "peak_rss_bytes": peak,
        "estimated_total_memory_bytes": estimated,
        "rss_to_estimated_total_ratio": ratio(rss, estimated),
        "peak_rss_to_estimated_total_ratio": ratio(peak, estimated),
        "estimated_total_to_rss_ratio": ratio(estimated, rss),
    })
}

fn estimate_ratio_error(
    sample: &Value,
    stats: &cortex_engine::StorageStats,
    args: &Args,
) -> Option<String> {
    let rss = sample["rss_bytes"].as_u64().unwrap_or(0);
    let estimated = stats.estimated_total_memory_bytes as u64;
    if estimated == 0 {
        return Some("estimated_total_memory_bytes is zero".to_owned());
    }
    let ratio = (rss as f64) / (estimated as f64);
    (ratio > args.max_rss_to_estimated_total_ratio).then(|| {
        format!(
            "rss_to_estimated_total_ratio exceeded threshold: {ratio:.3} > {:.3}",
            args.max_rss_to_estimated_total_ratio
        )
    })
}

fn payload_clone_gate_report() -> Value {
    let checks = [
        require_check(
            "crates/cortex-core/src/memtable/mod.rs",
            "pub fn visible_iter",
            "borrowed visible iterator",
        ),
        require_check(
            "crates/cortex-core/src/memtable/mod.rs",
            "pub fn visible_created_after_iter",
            "borrowed delta iterator",
        ),
        require_check(
            "crates/cortex-storage/src/segment.rs",
            "pub struct SegmentCellRef",
            "borrowed segment cell view",
        ),
        forbid_check(
            "crates/cortex-engine/src/checkpoint.rs",
            "self.snapshot_versions()",
            "checkpoint snapshot clone path",
        ),
        forbid_check(
            "crates/cortex-engine/src/verification.rs",
            "self.snapshot_versions()",
            "VERIFY FACT full clone scan",
        ),
        forbid_check(
            "crates/cortex-engine/src/verification.rs",
            "bind_aql_cached",
            "VERIFY FACT retrieval-index bind path",
        ),
        forbid_check(
            "crates/cortex-engine/src/verification/graph.rs",
            "conflicts_for_fact",
            "VERIFY graph enrichment full conflict-index scan",
        ),
    ];
    let checks = checks.into_iter().collect::<Vec<_>>();
    let passed = checks
        .iter()
        .all(|check| check["ok"].as_bool().unwrap_or(false));
    json!({
        "passed": passed,
        "method": "static_source_gate",
        "checks": checks,
    })
}

fn require_check(path: &str, needle: &str, label: &str) -> Value {
    source_check(path, needle, label, true)
}

fn forbid_check(path: &str, needle: &str, label: &str) -> Value {
    source_check(path, needle, label, false)
}

fn source_check(path: &str, needle: &str, label: &str, require: bool) -> Value {
    let text = fs::read_to_string(path).unwrap_or_default();
    let contains = text.contains(needle);
    let ok = if require { contains } else { !contains };
    json!({
        "ok": ok,
        "path": path,
        "label": label,
        "kind": if require { "require" } else { "forbid" },
        "needle": needle,
    })
}

fn clone_gate_errors(report: &Value) -> Vec<String> {
    report["checks"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|check| !check["ok"].as_bool().unwrap_or(false))
        .filter_map(|check| check["label"].as_str())
        .map(|label| format!("payload clone gate failed: {label}"))
        .collect()
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

fn ratio(numerator: u64, denominator: u64) -> f64 {
    if denominator == 0 {
        return 0.0;
    }
    round_ms((numerator as f64) / (denominator as f64))
}

fn round_ms(value: f64) -> f64 {
    (value * 1000.0).round() / 1000.0
}

struct Args {
    root: PathBuf,
    report: PathBuf,
    cells: usize,
    max_rss_to_estimated_total_ratio: f64,
}

impl Args {
    fn parse(values: impl Iterator<Item = String>) -> Result<Self, String> {
        let mut args = Self {
            root: PathBuf::from("target/memory-profile"),
            report: PathBuf::from("target/memory-profile/report.json"),
            cells: 10_000,
            max_rss_to_estimated_total_ratio: 128.0,
        };
        let mut values = values.peekable();
        while let Some(arg) = values.next() {
            match arg.as_str() {
                "--root" => args.root = PathBuf::from(next_value(&mut values, "--root")?),
                "--report" => args.report = PathBuf::from(next_value(&mut values, "--report")?),
                "--cells" => {
                    args.cells = parse_usize(next_value(&mut values, "--cells")?, "--cells")?
                }
                "--max-rss-to-estimated-total-ratio" => {
                    args.max_rss_to_estimated_total_ratio = parse_f64(
                        next_value(&mut values, "--max-rss-to-estimated-total-ratio")?,
                        "--max-rss-to-estimated-total-ratio",
                    )?
                }
                "--help" | "-h" => return Err(help_text()),
                unknown => return Err(format!("unknown argument: {unknown}\n{}", help_text())),
            }
        }
        if args.cells == 0 {
            return Err("--cells must be positive".to_owned());
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

fn parse_f64(value: String, flag: &str) -> Result<f64, String> {
    value
        .parse::<f64>()
        .map_err(|_| format!("invalid value for {flag}: {value}"))
}

fn help_text() -> String {
    "usage: memory_profile_check [--root PATH] [--report PATH] [--cells N] [--max-rss-to-estimated-total-ratio N]".to_owned()
}
