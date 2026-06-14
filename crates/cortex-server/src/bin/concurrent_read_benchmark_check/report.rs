use std::fs;
use std::path::Path;

use serde_json::{json, Value};

use super::args::Args;

pub(super) fn build_report(
    args: &Args,
    duration_ms: f64,
    modes: Vec<Value>,
    errors: Vec<String>,
) -> Value {
    json!({
        "schema_version": "cortexdb.concurrent_read_benchmark.v1",
        "ok": errors.is_empty(),
        "workload_class": "local_concurrent_read_throughput",
        "cells": args.cells,
        "read_ops_per_thread": args.read_ops_per_thread,
        "writer_ops": args.writer_ops,
        "reader_thread_counts": args.reader_threads,
        "duration_ms": duration_ms,
        "slo_thresholds": {
            "max_p95_ms": args.max_p95_ms,
        },
        "modes": modes,
        "comparisons": build_comparisons(&modes),
        "errors": errors,
    })
}

pub(super) fn collect_slo_errors(args: &Args, modes: &[Value]) -> Vec<String> {
    let mut errors = Vec::new();
    for mode in modes {
        let mode_name = mode
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        for point in mode
            .get("curve")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            let threads = point
                .get("reader_threads")
                .and_then(Value::as_u64)
                .unwrap_or(0);
            let read_p95 = point
                .get("reader_latency")
                .and_then(|latency| latency.get("p95_ms"))
                .and_then(Value::as_f64)
                .unwrap_or(0.0);
            if read_p95 > args.max_p95_ms {
                errors.push(format!(
                    "{mode_name} readers={threads} p95 exceeded threshold: {read_p95:.3} > {:.3}",
                    args.max_p95_ms
                ));
            }
            let write_p95 = point
                .get("writer_latency")
                .and_then(|latency| latency.get("p95_ms"))
                .and_then(Value::as_f64)
                .unwrap_or(0.0);
            if write_p95 > args.max_p95_ms {
                errors.push(format!(
                    "{mode_name} writer readers={threads} p95 exceeded threshold: {write_p95:.3} > {:.3}",
                    args.max_p95_ms
                ));
            }
        }
    }
    errors
}

pub(super) fn build_comparisons(modes: &[Value]) -> Vec<Value> {
    let Some(actor) = mode_curve(modes, "actor_route_shared") else {
        return Vec::new();
    };
    let Some(rwlock) = mode_curve(modes, "rwlock_direct") else {
        return Vec::new();
    };
    let mut comparisons = Vec::new();
    for actor_point in actor {
        let Some(threads) = actor_point.get("reader_threads").and_then(Value::as_u64) else {
            continue;
        };
        let Some(rwlock_point) = rwlock.iter().find(|point| {
            point
                .get("reader_threads")
                .and_then(Value::as_u64)
                .is_some_and(|value| value == threads)
        }) else {
            continue;
        };
        let actor_tps = read_throughput(actor_point);
        let rwlock_tps = read_throughput(rwlock_point);
        comparisons.push(json!({
            "reader_threads": threads,
            "actor_read_throughput_per_sec": round_ms(actor_tps),
            "rwlock_read_throughput_per_sec": round_ms(rwlock_tps),
            "rwlock_vs_actor_read_throughput_ratio": round_ratio(ratio(rwlock_tps, actor_tps)),
        }));
    }
    comparisons
}

pub(super) fn latency_summary(values: &[f64]) -> Value {
    json!({
        "count": values.len(),
        "p50_ms": round_ms(percentile(values, 0.50)),
        "p95_ms": round_ms(percentile(values, 0.95)),
        "p99_ms": round_ms(percentile(values, 0.99)),
        "max_ms": round_ms(values.iter().copied().fold(0.0, f64::max)),
    })
}

pub(super) fn round_ms(value: f64) -> f64 {
    (value * 1000.0).round() / 1000.0
}

pub(super) fn write_json_report(path: &Path, report: &Value) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    }
    let body = serde_json::to_string_pretty(report)
        .map_err(|error| format!("failed to encode JSON report: {error}"))?;
    fs::write(path, format!("{body}\n"))
        .map_err(|error| format!("failed to write {}: {error}", path.display()))
}

pub(super) fn write_markdown_report(path: &Path, report: &Value) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    }
    let mut lines = vec![
        "# Concurrent Read Benchmark".to_owned(),
        String::new(),
        format!("Status: `{}`", if report["ok"] == true { "passed" } else { "failed" }),
        String::new(),
        "| mode | readers | read ops | write ops | read ops/s | write ops/s | read p95 ms | write p95 ms |".to_owned(),
        "| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |".to_owned(),
    ];
    for mode in report
        .get("modes")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let mode_name = mode
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        for point in mode
            .get("curve")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            lines.push(format!(
                "| {mode_name} | {} | {} | {} | {:.3} | {:.3} | {:.3} | {:.3} |",
                point["reader_threads"].as_u64().unwrap_or(0),
                point["read_operations"].as_u64().unwrap_or(0),
                point["write_operations"].as_u64().unwrap_or(0),
                point["read_throughput_per_sec"].as_f64().unwrap_or(0.0),
                point["write_throughput_per_sec"].as_f64().unwrap_or(0.0),
                point["reader_latency"]["p95_ms"].as_f64().unwrap_or(0.0),
                point["writer_latency"]["p95_ms"].as_f64().unwrap_or(0.0),
            ));
        }
    }
    lines.extend([String::new(), "## Comparison".to_owned(), String::new()]);
    lines.push("| readers | actor read ops/s | rwlock read ops/s | rwlock/actor |".to_owned());
    lines.push("| ---: | ---: | ---: | ---: |".to_owned());
    for comparison in report
        .get("comparisons")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        lines.push(format!(
            "| {} | {:.3} | {:.3} | {:.3} |",
            comparison["reader_threads"].as_u64().unwrap_or(0),
            comparison["actor_read_throughput_per_sec"]
                .as_f64()
                .unwrap_or(0.0),
            comparison["rwlock_read_throughput_per_sec"]
                .as_f64()
                .unwrap_or(0.0),
            comparison["rwlock_vs_actor_read_throughput_ratio"]
                .as_f64()
                .unwrap_or(0.0),
        ));
    }
    fs::write(path, format!("{}\n", lines.join("\n")))
        .map_err(|error| format!("failed to write {}: {error}", path.display()))
}

pub(super) fn self_test() -> Result<(), String> {
    let summary = latency_summary(&[3.0, 1.0, 2.0, 4.0]);
    if summary["p50_ms"] != 2.0 || summary["p95_ms"] != 3.0 || summary["max_ms"] != 4.0 {
        return Err("latency summary self-test failed".to_owned());
    }
    let modes = vec![
        json!({"name": "actor_route_shared", "curve": [{"reader_threads": 1, "read_throughput_per_sec": 10.0}]}),
        json!({"name": "rwlock_direct", "curve": [{"reader_threads": 1, "read_throughput_per_sec": 25.0}]}),
    ];
    let comparisons = build_comparisons(&modes);
    if comparisons
        .first()
        .and_then(|value| value["rwlock_vs_actor_read_throughput_ratio"].as_f64())
        != Some(2.5)
    {
        return Err("comparison self-test failed".to_owned());
    }
    Ok(())
}

fn mode_curve<'a>(modes: &'a [Value], name: &str) -> Option<&'a Vec<Value>> {
    modes.iter().find_map(|mode| {
        (mode.get("name").and_then(Value::as_str) == Some(name))
            .then(|| mode.get("curve").and_then(Value::as_array))
            .flatten()
    })
}

fn read_throughput(point: &Value) -> f64 {
    point
        .get("read_throughput_per_sec")
        .and_then(Value::as_f64)
        .unwrap_or(0.0)
}

fn ratio(current: f64, previous: f64) -> f64 {
    if previous <= 0.0 {
        0.0
    } else {
        current / previous
    }
}

fn round_ratio(value: f64) -> f64 {
    (value * 1_000_000.0).round() / 1_000_000.0
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
