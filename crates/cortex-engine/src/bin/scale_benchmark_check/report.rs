use std::fs;
use std::time::Instant;

use serde_json::json;

use crate::args::Args;
use crate::metrics::{matrix_from_phases, round_ms};

pub(crate) fn write_report(
    args: &Args,
    started: Instant,
    phases: &[serde_json::Value],
    validation: &cortex_engine::validation::StorageValidationReport,
    errors: Vec<String>,
) -> Result<(), String> {
    let report = json!({
        "schema_version": "cortexdb.scale_benchmark.v1",
        "ok": errors.is_empty(),
        "cells": args.cells,
        "payload_profile": args.payload_bytes
            .map(|bytes| format!("fixed_{bytes}b"))
            .unwrap_or_else(|| "realistic_0_5kb_to_4kb".to_owned()),
        "fixture_mode": fixture_mode(args),
        "samples": {
            "read": args.samples,
            "search": args.search_samples,
            "context": args.context_samples,
            "verify": args.verify_samples,
        },
        "duration_ms": round_ms(started.elapsed().as_secs_f64() * 1000.0),
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

fn fixture_mode(args: &Args) -> &'static str {
    if args.reopen_only {
        "reopen_only"
    } else if args.direct_checkpoint {
        "direct_checkpoint"
    } else {
        "wal_put_checkpoint"
    }
}
