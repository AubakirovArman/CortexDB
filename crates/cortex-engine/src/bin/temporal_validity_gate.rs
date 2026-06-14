use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Instant;

use cortex_aql::{AgentId, AgentView, BrainId, MemoryType, RetrievalMode, Q16_ZERO};
use cortex_core::CellId;
use cortex_engine::{scope_id, Database, DatabaseOptions, PayloadResidency};
use serde_json::json;

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
    let args = Args::parse(std::env::args().skip(1))?;
    reset_target_root(&args.root)?;
    let started = Instant::now();
    let valid_expected = prepare_database(&args)?;
    let db = Database::open_with_options(
        &args.root,
        DatabaseOptions {
            payload_residency: args.payload_residency,
            payload_cache_bytes: 0,
            ..DatabaseOptions::default()
        },
    )
    .map_err(|error| format!("open lazy database: {error}"))?;
    let query_started = Instant::now();
    let results = db
        .retrieve_aql(
            r#"RETRIEVE CONTEXT FOR TASK "temporal budget" IN BRAIN default
WHERE space = default AND status = "ready" BUDGET 1000000 TOKENS LIMIT 100000 CANDIDATES
REQUIRE valid at "2025-06-01";"#,
            &view(args.cells as u32),
        )
        .map_err(|error| format!("retrieve valid-at query: {error}"))?;
    let segment_loads = db.payload_cache_stats().segment_loads;
    let max_allowed_segment_loads = valid_expected as u64 + 4;
    let ok = results.len() == valid_expected && segment_loads <= max_allowed_segment_loads;
    let debug_counts = (!ok)
        .then(|| debug_counts(&db, args.cells as u32))
        .transpose()?;
    write_report(ReportInput {
        args: &args,
        valid_expected,
        returned_cells: results.len(),
        segment_loads,
        max_allowed_segment_loads,
        query_elapsed_ms: query_started.elapsed().as_millis(),
        elapsed_ms: started.elapsed().as_millis(),
        debug_counts,
        ok,
    })?;
    ok.then_some(()).ok_or_else(|| {
        format!(
            "temporal validity gate failed: expected {valid_expected}, got {}, segment_loads={segment_loads}",
            results.len()
        )
    })
}

fn prepare_database(args: &Args) -> Result<usize, String> {
    let mut db = Database::open(&args.root).map_err(|error| format!("open database: {error}"))?;
    let mut valid_expected = 0usize;
    for start in (1..=args.cells).step_by(args.batch_size) {
        let end = (start + args.batch_size - 1).min(args.cells);
        let mut batch = Vec::with_capacity(end - start + 1);
        for id in start..=end {
            let payload = temporal_payload(id);
            if id.is_multiple_of(1_000) {
                valid_expected += 1;
            }
            batch.push((CellId(id as u64), payload));
        }
        db.put_cells(batch)
            .map_err(|error| format!("put temporal batch {start}-{end}: {error}"))?;
    }
    db.checkpoint()
        .map_err(|error| format!("checkpoint temporal corpus: {error}"))?;
    drop(db);
    Ok(valid_expected)
}

fn temporal_payload(id: usize) -> Vec<u8> {
    let validity = if id.is_multiple_of(1_000) {
        "valid_from=2025-01-01\nvalid_to=2025-12-31"
    } else if id.is_multiple_of(2) {
        "valid_to=2024-12-31"
    } else {
        "valid_from=2026-01-01"
    };
    format!("scope=default\nstatus=ready\n{validity}\n\ntemporal budget cell {id}").into_bytes()
}

fn view(candidate_limit: u32) -> AgentView {
    AgentView {
        agent_id: AgentId(1),
        label: Some("temporal-validity-gate".to_owned()),
        readable_brains: BTreeSet::from([BrainId(1)]),
        readable_scopes: BTreeSet::from([scope_id("default")]),
        writable_scopes: BTreeSet::new(),
        allowed_modes: BTreeSet::from([RetrievalMode::Balanced]),
        allowed_memory_types: BTreeSet::from([MemoryType::Decision]),
        max_context_budget_tokens: 1_000_000,
        default_context_budget_tokens: 1_000_000,
        max_candidate_limit: candidate_limit,
        default_candidate_limit: candidate_limit,
        min_required_confidence_q16: Q16_ZERO,
        max_ttl_seconds: None,
        allow_remember: false,
        allow_verify_fact: false,
        allow_audit_mode: false,
        require_citations_by_default: false,
        private_scope: None,
    }
}

fn debug_counts(db: &Database, candidate_limit: u32) -> Result<serde_json::Value, String> {
    let view = view(candidate_limit);
    let unfiltered = db
        .retrieve_aql(
            r#"RETRIEVE CONTEXT FOR TASK "temporal budget" IN BRAIN default
BUDGET 1000000 TOKENS LIMIT 100000 CANDIDATES;"#,
            &view,
        )
        .map_err(|error| format!("debug unfiltered retrieve: {error}"))?
        .len();
    let without_valid = db
        .retrieve_aql(
            r#"RETRIEVE CONTEXT FOR TASK "temporal budget" IN BRAIN default
WHERE space = default AND status = "ready" BUDGET 1000000 TOKENS LIMIT 100000 CANDIDATES;"#,
            &view,
        )
        .map_err(|error| format!("debug without-valid retrieve: {error}"))?
        .len();
    Ok(json!({
        "unfiltered": unfiltered,
        "without_valid": without_valid,
    }))
}

struct ReportInput<'a> {
    args: &'a Args,
    valid_expected: usize,
    returned_cells: usize,
    segment_loads: u64,
    max_allowed_segment_loads: u64,
    query_elapsed_ms: u128,
    elapsed_ms: u128,
    debug_counts: Option<serde_json::Value>,
    ok: bool,
}

fn write_report(input: ReportInput<'_>) -> Result<(), String> {
    let report = json!({
        "schema_version": "cortexdb.temporal_validity_gate.v1",
        "cells": input.args.cells,
        "payload_residency": format!("{:?}", input.args.payload_residency),
        "valid_expected": input.valid_expected,
        "returned_cells": input.returned_cells,
        "segment_loads_after_query": input.segment_loads,
        "max_allowed_segment_loads": input.max_allowed_segment_loads,
        "query_elapsed_ms": input.query_elapsed_ms,
        "elapsed_ms": input.elapsed_ms,
        "debug_counts": input.debug_counts,
        "ok": input.ok,
    });
    if let Some(parent) = input.args.report.parent() {
        std::fs::create_dir_all(parent).map_err(|error| format!("create report dir: {error}"))?;
    }
    std::fs::write(
        &input.args.report,
        serde_json::to_string_pretty(&report).map_err(|error| error.to_string())? + "\n",
    )
    .map_err(|error| format!("write report {}: {error}", input.args.report.display()))
}

fn reset_target_root(root: &Path) -> Result<(), String> {
    if !root.starts_with("target") {
        return Err(format!(
            "refusing to delete non-target root {}; pass a target/... path",
            root.display()
        ));
    }
    if root.exists() {
        std::fs::remove_dir_all(root).map_err(|error| format!("remove root: {error}"))?;
    }
    std::fs::create_dir_all(root).map_err(|error| format!("create root: {error}"))
}

struct Args {
    root: PathBuf,
    report: PathBuf,
    cells: usize,
    batch_size: usize,
    payload_residency: PayloadResidency,
}

impl Args {
    fn parse(values: impl Iterator<Item = String>) -> Result<Self, String> {
        let mut args = Self {
            root: PathBuf::from("target/temporal-validity-gate/100k/db"),
            report: PathBuf::from("target/temporal-validity-gate/100k/report.json"),
            cells: 100_000,
            batch_size: 5_000,
            payload_residency: PayloadResidency::Lazy,
        };
        let mut values = values.peekable();
        while let Some(arg) = values.next() {
            match arg.as_str() {
                "--root" => args.root = PathBuf::from(next_value(&mut values, "--root")?),
                "--report" => args.report = PathBuf::from(next_value(&mut values, "--report")?),
                "--cells" => {
                    args.cells = parse_usize(next_value(&mut values, "--cells")?, "--cells")?
                }
                "--batch-size" => {
                    args.batch_size =
                        parse_usize(next_value(&mut values, "--batch-size")?, "--batch-size")?
                }
                "--payload-residency" => {
                    args.payload_residency =
                        parse_payload_residency(next_value(&mut values, "--payload-residency")?)?
                }
                "--help" | "-h" => return Err(help_text()),
                unknown => return Err(format!("unknown argument: {unknown}\n{}", help_text())),
            }
        }
        if args.cells == 0 || args.batch_size == 0 {
            return Err("--cells and --batch-size must be positive".to_owned());
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

fn parse_payload_residency(value: String) -> Result<PayloadResidency, String> {
    match value.as_str() {
        "memory" => Ok(PayloadResidency::Memory),
        "lazy" => Ok(PayloadResidency::Lazy),
        _ => Err(format!(
            "invalid value for --payload-residency: {value}; expected memory or lazy"
        )),
    }
}

fn help_text() -> String {
    "usage: temporal_validity_gate [--root target/.../db] [--report target/.../report.json] [--cells N] [--batch-size N] [--payload-residency memory|lazy]".to_owned()
}
