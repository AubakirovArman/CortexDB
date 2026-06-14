use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Instant;

use cortex_core::memtable::CellVersion;
use cortex_core::{CellId, CommitSeq, KnowledgeCell, KnowledgeCellMetadata, KnowledgeCellType};
use cortex_engine::graph::KnowledgeGraphIndex;
use serde_json::json;

#[derive(Clone, Debug)]
struct Args {
    report: PathBuf,
    nodes: usize,
    samples: usize,
    max_hops: u32,
    visit_budget: usize,
    max_p95_ms: f64,
}

impl Default for Args {
    fn default() -> Self {
        Self {
            report: PathBuf::from("target/graph-index-performance/report.json"),
            nodes: 100_000,
            samples: 31,
            max_hops: 16,
            visit_budget: 4_096,
            max_p95_ms: 50.0,
        }
    }
}

fn main() -> ExitCode {
    match run(parse_args()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: Args) -> Result<(), String> {
    let build_started = Instant::now();
    let index = KnowledgeGraphIndex::from_versions(build_versions(args.nodes));
    let build_ms = elapsed_ms(build_started);

    let mut latencies = Vec::with_capacity(args.samples);
    let mut visited_edges = Vec::with_capacity(args.samples);
    let mut budget_exceeded_samples = 0usize;
    for sample in 0..args.samples {
        let seed = seed_name(sample, args.nodes, args.max_hops);
        let started = Instant::now();
        let report =
            index.retrieve_related_cells_with_budget(&seed, args.max_hops, args.visit_budget);
        latencies.push(elapsed_ms(started));
        visited_edges.push(report.visited_edges as f64);
        if report.budget_exceeded {
            budget_exceeded_samples += 1;
        }
        if report.hits.is_empty() {
            return Err(format!("graph traversal returned no hits for {seed}"));
        }
    }

    let p95_ms = percentile(&latencies, 95);
    let status = if p95_ms <= args.max_p95_ms && budget_exceeded_samples == 0 {
        "passed"
    } else {
        "failed"
    };
    let payload = json!({
        "schema_version": "cortexdb.graph_index_performance.v1",
        "status": status,
        "nodes": args.nodes,
        "edges": args.nodes.saturating_sub(1),
        "samples": args.samples,
        "max_hops": args.max_hops,
        "visit_budget": args.visit_budget,
        "build_ms": build_ms,
        "latency_ms": {
            "p50": percentile(&latencies, 50),
            "p95": p95_ms,
            "max": percentile(&latencies, 100)
        },
        "visited_edges": {
            "p95": percentile(&visited_edges, 95),
            "max": percentile(&visited_edges, 100)
        },
        "budget_exceeded_samples": budget_exceeded_samples,
        "max_p95_ms": args.max_p95_ms
    });
    if let Some(parent) = args.report.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    fs::write(
        &args.report,
        serde_json::to_vec_pretty(&payload).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    if status == "passed" {
        println!(
            "graph index performance check passed: {}",
            args.report.display()
        );
        Ok(())
    } else {
        Err(format!(
            "graph index performance check failed: p95={p95_ms:.3}ms budget_exceeded_samples={budget_exceeded_samples}"
        ))
    }
}

fn build_versions(nodes: usize) -> Vec<CellVersion> {
    let mut versions = Vec::with_capacity(nodes.saturating_mul(2).saturating_sub(1));
    for index in 0..nodes {
        versions.push(cell_version(
            CellId(index as u64 + 1),
            KnowledgeCellType::Entity,
            format!("name={}\nkind=graph-node", node_name(index)),
        ));
    }
    for index in 0..nodes.saturating_sub(1) {
        versions.push(cell_version(
            CellId(nodes as u64 + index as u64 + 1),
            KnowledgeCellType::Relation,
            format!(
                "subject={}\npredicate=next\nobject={}",
                node_name(index),
                node_name(index + 1)
            ),
        ));
    }
    versions
}

fn cell_version(cell_id: CellId, cell_type: KnowledgeCellType, body: String) -> CellVersion {
    let payload = KnowledgeCell::new(
        KnowledgeCellMetadata {
            scope: "graph:performance".to_owned(),
            status: "ready".to_owned(),
            cell_type,
            ..KnowledgeCellMetadata::default()
        },
        body,
    )
    .encode_payload();
    CellVersion::new(cell_id, CommitSeq(cell_id.0), payload, 0)
}

fn seed_name(sample: usize, nodes: usize, max_hops: u32) -> String {
    let tail_room = max_hops as usize + 1;
    let span = nodes.saturating_sub(tail_room).max(1);
    node_name(sample.saturating_mul(9_973) % span)
}

fn node_name(index: usize) -> String {
    format!("node-{index}")
}

fn percentile(values: &[f64], percentile: usize) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(|left, right| left.total_cmp(right));
    let rank = sorted.len().saturating_mul(percentile).div_ceil(100);
    sorted[rank.saturating_sub(1)]
}

fn elapsed_ms(started: Instant) -> f64 {
    started.elapsed().as_secs_f64() * 1_000.0
}

fn parse_args() -> Args {
    let mut args = Args::default();
    let mut raw = env::args().skip(1);
    while let Some(arg) = raw.next() {
        let Some(value) = raw.next() else {
            continue;
        };
        match arg.as_str() {
            "--report" => args.report = PathBuf::from(value),
            "--nodes" => args.nodes = value.parse().unwrap_or(args.nodes),
            "--samples" => args.samples = value.parse().unwrap_or(args.samples),
            "--max-hops" => args.max_hops = value.parse().unwrap_or(args.max_hops),
            "--visit-budget" => args.visit_budget = value.parse().unwrap_or(args.visit_budget),
            "--max-p95-ms" => args.max_p95_ms = value.parse().unwrap_or(args.max_p95_ms),
            _ => {}
        }
    }
    args
}
