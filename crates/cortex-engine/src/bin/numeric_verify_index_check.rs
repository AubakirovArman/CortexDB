use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Instant;

use cortex_aql::{AgentId, AgentView, BrainId, MemoryType, RetrievalMode, ScopeId, Q16_ZERO};
use cortex_core::{CellDescriptor, CellId};
use cortex_engine::{scope_id, Database, EngineAqlIndex};
use cortex_storage::hnsw::HnswGraphIndex;
use cortex_storage::manifest::{ManifestSegment, StorageManifest};
use cortex_storage::segment::{SegmentCellRef, SegmentWriter};
use cortex_storage::vectors::VectorIndex;
use serde_json::{json, Value};

const SCOPE: &str = "c13:numeric";
const TARGET_PROJECT: &str = "c13targetproject";
const TARGET_METRIC: &str = "c13targetmetric";
const TARGET_FACT: &str = "c13targetproject c13targetmetric is 12 KZT";
const TARGET_AQL: &str =
    r#"VERIFY FACT "c13targetproject c13targetmetric is 12 KZT" IN BRAIN default;"#;

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
    phases.push(prepare_direct_checkpoint(&db_path, &args)?);

    let (db, open_phase) = measure_once("open_prepared", 1, || Database::open(&db_path))?;
    phases.push(open_phase);
    let view = bench_view();
    verify_target(&db, &view)?;

    let mut latencies = Vec::with_capacity(args.samples);
    for _ in 0..args.samples {
        let sample_started = Instant::now();
        verify_target(&db, &view)?;
        latencies.push(sample_started.elapsed().as_secs_f64() * 1000.0);
    }
    let verify_latency = latency_summary(&latencies);
    let p95_ms = verify_latency["p95_ms"].as_f64().unwrap_or(f64::MAX);
    phases.push(json!({
        "name": "numeric_verify_fact",
        "samples": args.samples,
        "latency": verify_latency,
    }));

    let mut errors = Vec::new();
    if p95_ms > args.max_p95_ms {
        errors.push(format!(
            "numeric verify p95 exceeded threshold: {p95_ms:.3} > {:.3}",
            args.max_p95_ms
        ));
    }
    if !args.skip_validation {
        let validation = db.validate_storage_report();
        if !validation.errors.is_empty() {
            errors.push(format!(
                "storage validation failed: {}",
                validation.errors.join("; ")
            ));
        }
    }

    let report = json!({
        "schema_version": "cortexdb.numeric_verify_index.v1",
        "ok": errors.is_empty(),
        "cells": args.cells,
        "workload_class": "metric_sorted_numeric_verify_index",
        "query": TARGET_AQL,
        "target_fact": TARGET_FACT,
        "slo_thresholds": {
            "max_p95_ms": args.max_p95_ms,
        },
        "duration_ms": round_ms(started.elapsed().as_secs_f64() * 1000.0),
        "phases": phases,
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
            "numeric verify index check failed: {}",
            args.report.display()
        ));
    }
    println!(
        "numeric verify index check passed: {}",
        args.report.display()
    );
    Ok(())
}

fn verify_target(db: &Database, view: &AgentView) -> Result<(), String> {
    let report = db
        .verify_fact_aql(TARGET_AQL, view)
        .map_err(|error| error.to_string())?;
    if !report.evidence.iter().any(|item| item.cell_id == CellId(1)) {
        return Err("numeric verify did not return the indexed support cell".to_owned());
    }
    if !report
        .contradicting_evidence
        .iter()
        .any(|item| item.cell_id == CellId(2))
    {
        return Err("numeric verify did not return the indexed conflict cell".to_owned());
    }
    if !report
        .numeric_conflicts
        .iter()
        .any(|item| item.cell_id == CellId(2) && item.metric == TARGET_METRIC)
    {
        return Err("numeric verify did not emit the typed numeric conflict".to_owned());
    }
    Ok(())
}

fn prepare_direct_checkpoint(db_path: &Path, args: &Args) -> Result<Value, String> {
    let started = Instant::now();
    fs::create_dir_all(segments_path(db_path))
        .map_err(|error| format!("failed to create {}: {error}", db_path.display()))?;
    let mut manifest = StorageManifest::default();
    let mut next = 1usize;
    let mut segment_id = 1u64;
    while next <= args.cells {
        let end = next
            .saturating_add(args.batch_size)
            .saturating_sub(1)
            .min(args.cells);
        write_segment_batch(db_path, segment_id, next, end)?;
        manifest.checkpoint_segment(ManifestSegment {
            id: segment_id,
            generation: segment_id,
            checkpoint_seq: end as u64,
            cell_count: u32::try_from(end - next + 1)
                .map_err(|_| "segment cell_count exceeds u32".to_owned())?,
        });
        next = end + 1;
        segment_id += 1;
    }
    manifest
        .store(manifest_path(db_path))
        .map_err(|error| format!("failed to store manifest: {error}"))?;
    let elapsed_ms = started.elapsed().as_secs_f64() * 1000.0;
    Ok(json!({
        "name": "direct_checkpoint",
        "units": args.cells,
        "segments": manifest.live_segments.len(),
        "batch_size": args.batch_size,
        "elapsed_ms": round_ms(elapsed_ms),
        "throughput_per_sec": throughput(args.cells, elapsed_ms),
    }))
}

fn write_segment_batch(
    db_path: &Path,
    segment_id: u64,
    start: usize,
    end: usize,
) -> Result<(), String> {
    let payloads = (start..=end).map(payload).collect::<Vec<_>>();
    let refs = payloads
        .iter()
        .enumerate()
        .map(|(offset, payload)| {
            let index = start + offset;
            let candidate_id =
                u32::try_from(index).map_err(|_| "candidate id exceeds u32".to_owned())?;
            Ok(SegmentCellRef {
                candidate_id,
                cell_id: index as u64,
                created_seq: index as u64,
                deleted_seq: None,
                descriptor: Some(CellDescriptor::from_payload_lossy(payload).encode_section_v1()),
                payload,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    SegmentWriter::write_refs(segment_path(db_path, segment_id), &refs)
        .map_err(|error| format!("failed to write segment {segment_id}: {error}"))?;
    let index = EngineAqlIndex::try_from_segment_cell_refs(&refs)
        .map_err(|error| format!("failed to build index {segment_id}: {error}"))?;
    index
        .bitmap_index()
        .write(bitmap_path(db_path, segment_id))
        .map_err(|error| format!("failed to write bitmap index {segment_id}: {error}"))?;
    index
        .lexical_index()
        .write(lexical_path(db_path, segment_id))
        .map_err(|error| format!("failed to write lexical index {segment_id}: {error}"))?;
    VectorIndex::default()
        .write(vector_path(db_path, segment_id))
        .map_err(|error| format!("failed to write vector index {segment_id}: {error}"))?;
    HnswGraphIndex::default()
        .write(hnsw_path(db_path, segment_id))
        .map_err(|error| format!("failed to write hnsw graph {segment_id}: {error}"))?;
    Ok(())
}

fn payload(index: usize) -> Vec<u8> {
    let (project, metric, value) = match index {
        1 => (
            TARGET_PROJECT.to_owned(),
            TARGET_METRIC.to_owned(),
            "12 KZT".to_owned(),
        ),
        2 => (
            TARGET_PROJECT.to_owned(),
            TARGET_METRIC.to_owned(),
            "14 KZT".to_owned(),
        ),
        _ => (
            format!("c13project{index:07}"),
            format!("c13metric{index:07}"),
            format!("{} KZT", 100 + index % 10_000),
        ),
    };
    format!(
        "scope={SCOPE}\nstatus=ready\ntype=fact\nsource=c13-numeric-{index}\nsource_trust_q16=60000\n\nproject={project}\nmetric={metric}\nvalue={value}\n{project} {metric} is {value}."
    )
    .into_bytes()
}

fn bench_view() -> AgentView {
    AgentView {
        agent_id: AgentId(1),
        label: Some("numeric-verify-index".to_owned()),
        readable_brains: BTreeSet::from([BrainId(1)]),
        readable_scopes: BTreeSet::from([scope_id(SCOPE)]),
        writable_scopes: BTreeSet::from([scope_id(SCOPE)]),
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

fn throughput(units: usize, elapsed_ms: f64) -> f64 {
    if elapsed_ms <= 0.0 {
        0.0
    } else {
        round_ms((units as f64) / (elapsed_ms / 1000.0))
    }
}

fn round_ms(value: f64) -> f64 {
    (value * 1000.0).round() / 1000.0
}

fn manifest_path(db_path: &Path) -> PathBuf {
    db_path.join("manifest.acm")
}

fn segments_path(db_path: &Path) -> PathBuf {
    db_path.join("segments")
}

fn segment_path(db_path: &Path, id: u64) -> PathBuf {
    segments_path(db_path).join(format!("segment-{id}.acs"))
}

fn bitmap_path(db_path: &Path, id: u64) -> PathBuf {
    segments_path(db_path).join(format!("segment-{id}.acb"))
}

fn lexical_path(db_path: &Path, id: u64) -> PathBuf {
    segments_path(db_path).join(format!("segment-{id}.aci"))
}

fn vector_path(db_path: &Path, id: u64) -> PathBuf {
    segments_path(db_path).join(format!("segment-{id}.acv"))
}

fn hnsw_path(db_path: &Path, id: u64) -> PathBuf {
    segments_path(db_path).join(format!("segment-{id}.ach"))
}

#[derive(Clone, Debug, PartialEq)]
struct Args {
    root: PathBuf,
    report: PathBuf,
    cells: usize,
    samples: usize,
    batch_size: usize,
    max_p95_ms: f64,
    skip_validation: bool,
}

impl Args {
    fn parse(values: impl Iterator<Item = String>) -> Result<Self, String> {
        let mut args = Self {
            root: PathBuf::from("target/numeric-verify-index"),
            report: PathBuf::from("target/numeric-verify-index/report.json"),
            cells: 100_000,
            samples: 25,
            batch_size: 50_000,
            max_p95_ms: 100.0,
            skip_validation: false,
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
                "--batch-size" => {
                    args.batch_size =
                        parse_usize(next_value(&mut values, "--batch-size")?, "--batch-size")?
                }
                "--max-p95-ms" => {
                    args.max_p95_ms = parse_f64(next_value(&mut values, "--max-p95-ms")?)?
                }
                "--skip-validation" => args.skip_validation = true,
                "--help" | "-h" => return Err(help_text()),
                unknown => return Err(format!("unknown argument: {unknown}\n{}", help_text())),
            }
        }
        for (name, value) in [
            ("--cells", args.cells),
            ("--samples", args.samples),
            ("--batch-size", args.batch_size),
        ] {
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

fn parse_f64(value: String) -> Result<f64, String> {
    value
        .parse::<f64>()
        .map_err(|_| format!("invalid value for --max-p95-ms: {value}"))
}

fn help_text() -> String {
    "usage: numeric_verify_index_check [--root PATH] [--report PATH] [--cells N] [--samples N] [--batch-size N] [--max-p95-ms MS] [--skip-validation]".to_owned()
}
