use std::env;
use std::fs;
use std::process::ExitCode;
use std::time::Instant;

use cortex_core::CellId;
use cortex_engine::{Database, DatabaseOptions, PayloadResidency};
use serde_json::{json, Value};

#[path = "memory_profile_check/args.rs"]
mod args;
#[path = "memory_profile_check/latency.rs"]
mod latency;
#[path = "memory_profile_check/payload_gate.rs"]
mod payload_gate;

use args::Args;
use latency::get_latest_latency_report;
use payload_gate::{clone_gate_errors, payload_clone_gate_report};

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
    let db_path = args.root.join("db");
    if args.reopen_only {
        if !db_path.exists() {
            return Err(format!(
                "--reopen-only requires existing database at {}",
                db_path.display()
            ));
        }
    } else {
        fs::remove_dir_all(&args.root).ok();
        fs::create_dir_all(&args.root)
            .map_err(|error| format!("failed to create {}: {error}", args.root.display()))?;
    }

    let clone_gate = payload_clone_gate_report();
    let started = Instant::now();
    let mut samples = Vec::new();
    samples.push(memory_sample("process_start"));

    let options = DatabaseOptions {
        payload_residency: args.payload_residency,
        ..DatabaseOptions::default()
    };
    let mut after_put_stats = None;
    let mut after_checkpoint_stats = None;

    if !args.reopen_only {
        let mut db =
            Database::open_with_options(&db_path, options).map_err(|error| error.to_string())?;
        samples.push(memory_sample("open_empty"));

        db.put_cells(build_cells(args.cells, args.payload_bytes))
            .map_err(|error| error.to_string())?;
        after_put_stats = Some(db.storage_stats().map_err(|error| error.to_string())?);
        samples.push(memory_sample("after_put"));

        db.checkpoint().map_err(|error| error.to_string())?;
        after_checkpoint_stats = Some(db.storage_stats().map_err(|error| error.to_string())?);
        samples.push(memory_sample("after_checkpoint"));
        drop(db);
        samples.push(memory_sample("after_close"));
    }

    let db = Database::open_with_options(&db_path, options).map_err(|error| error.to_string())?;
    let after_reopen_stats = db.storage_stats().map_err(|error| error.to_string())?;
    samples.push(memory_sample("after_reopen"));
    let get_latest_latency = get_latest_latency_report(&db, args.cells, args.read_samples)?;

    let validation = db.validate_storage_report();
    if !validation.errors.is_empty() {
        return Err(format!(
            "storage validation failed: {}",
            validation.errors.join("; ")
        ));
    }

    let final_sample = memory_sample("final");
    let mut errors = clone_gate_errors(&clone_gate);
    if let Some(error) = estimate_ratio_error(&final_sample, &after_reopen_stats, &args) {
        errors.push(error);
    }

    let report = json!({
        "schema_version": "cortexdb.memory_profile.v1",
        "ok": errors.is_empty(),
        "cells": args.cells,
        "payload_bytes": args.payload_bytes,
        "mode": if args.reopen_only { "reopen_only" } else { "build_and_reopen" },
        "payload_residency": payload_residency_name(args.payload_residency),
        "duration_ms": round_ms(started.elapsed().as_secs_f64() * 1000.0),
        "resource_samples": samples,
        "final_resource_usage": final_sample,
        "storage_estimates": {
            "after_put": after_put_stats.as_ref().map(storage_estimate_report),
            "after_checkpoint": after_checkpoint_stats.as_ref().map(storage_estimate_report),
            "after_reopen": storage_estimate_report(&after_reopen_stats),
        },
        "latency": {
            "get_latest": get_latest_latency,
        },
        "estimate_vs_rss": estimate_vs_rss_report(&final_sample, &after_reopen_stats),
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

fn build_cells(cells: usize, payload_bytes: usize) -> Vec<(CellId, Vec<u8>)> {
    (1..=cells)
        .map(|index| (CellId(index as u64), build_payload(index, payload_bytes)))
        .collect()
}

fn build_payload(index: usize, payload_bytes: usize) -> Vec<u8> {
    let mut payload = format!(
        "scope=memory:profile\nstatus=ready\ntype=fact\nsource=memory-profile-{index}\n\nmemory profile payload {index} alpha beta gamma"
    )
    .into_bytes();
    if payload_bytes <= payload.len() {
        return payload;
    }

    payload.push(b'\n');
    let filler = format!("filler-{index:08}-");
    while payload.len() < payload_bytes {
        payload.extend_from_slice(filler.as_bytes());
    }
    payload.truncate(payload_bytes);
    payload
}

fn payload_residency_name(payload_residency: PayloadResidency) -> &'static str {
    match payload_residency {
        PayloadResidency::Memory => "memory",
        PayloadResidency::Lazy => "lazy",
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_payload_keeps_legacy_size_when_disabled() {
        let payload = build_payload(7, 0);
        let text = String::from_utf8(payload).unwrap();

        assert!(text.contains("source=memory-profile-7"));
        assert!(text.contains("memory profile payload 7 alpha beta gamma"));
    }

    #[test]
    fn build_payload_pads_to_requested_size() {
        let payload = build_payload(7, 4096);

        assert_eq!(payload.len(), 4096);
        assert!(payload.starts_with(b"scope=memory:profile\n"));
    }
}
