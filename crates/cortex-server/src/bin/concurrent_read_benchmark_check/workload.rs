use std::fs;
use std::path::Path;
use std::sync::{Arc, Barrier, RwLock};
use std::thread;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use cortex_core::CellId;
use cortex_engine::concurrency::WriterPrefRwLock;
use cortex_engine::Database;
use cortex_server::route_shared;
use serde_json::{json, Value};

use super::args::Args;
use super::report::{latency_summary, round_ms};
use super::support::{bench_view, ensure_aql_response, ensure_write_response, payload};

const QUERY: &str = r#"RETRIEVE CONTEXT FOR TASK "budget" IN BRAIN default WHERE space = bench AND status = "ready" LIMIT 10 CANDIDATES;"#;

#[derive(Clone, Copy)]
pub(super) enum Mode {
    ActorRouteShared,
    RwLockDirect,
}

impl Mode {
    fn name(self) -> &'static str {
        match self {
            Self::ActorRouteShared => "actor_route_shared",
            Self::RwLockDirect => "rwlock_direct",
        }
    }

    fn description(self) -> &'static str {
        match self {
            Self::ActorRouteShared => "HTTP route path through the legacy shared RwLock wrapper",
            Self::RwLockDirect => "direct engine reads under WriterPrefRwLock read guards",
        }
    }
}

pub(super) fn run_mode(mode: Mode, args: &Args, root: &Path) -> Result<Value, String> {
    let mut curve = Vec::new();
    for reader_threads in &args.reader_threads {
        let db_path = root.join(format!(
            "{}-{}-{}",
            mode.name(),
            reader_threads,
            unique_id()
        ));
        let point = match mode {
            Mode::ActorRouteShared => run_actor_scenario(args, &db_path, *reader_threads)?,
            Mode::RwLockDirect => run_rwlock_scenario(args, &db_path, *reader_threads)?,
        };
        curve.push(point);
    }
    Ok(json!({
        "name": mode.name(),
        "description": mode.description(),
        "curve": curve,
    }))
}

fn run_actor_scenario(args: &Args, db_path: &Path, readers: usize) -> Result<Value, String> {
    let db = seeded_database(db_path, args.cells)?;
    let shared = Arc::new(RwLock::new(db));
    let barrier = Arc::new(Barrier::new(readers + 2));
    let mut reader_handles = Vec::new();
    for _ in 0..readers {
        let shared = Arc::clone(&shared);
        let barrier = Arc::clone(&barrier);
        let read_ops = args.read_ops_per_thread;
        reader_handles.push(thread::spawn(move || {
            barrier.wait();
            let mut latencies = Vec::with_capacity(read_ops);
            for _ in 0..read_ops {
                let started = Instant::now();
                let response =
                    route_shared(&shared, "POST", "/v1/aql?scope=bench", QUERY.as_bytes())
                        .map_err(|error| error.to_string())?;
                ensure_aql_response(&response)?;
                latencies.push(started.elapsed().as_secs_f64() * 1000.0);
            }
            Ok::<_, String>(latencies)
        }));
    }
    let writer = {
        let shared = Arc::clone(&shared);
        let barrier = Arc::clone(&barrier);
        let cells = args.cells;
        let writer_ops = args.writer_ops;
        thread::spawn(move || {
            barrier.wait();
            let mut latencies = Vec::with_capacity(writer_ops);
            for offset in 0..writer_ops {
                let cell_id = cells + offset + 1;
                let target = format!("/v1/cell?cell_id={cell_id}");
                let payload = payload(cell_id);
                let started = Instant::now();
                let response = route_shared(&shared, "POST", &target, &payload)
                    .map_err(|error| error.to_string())?;
                ensure_write_response(&response)?;
                latencies.push(started.elapsed().as_secs_f64() * 1000.0);
            }
            Ok::<_, String>(latencies)
        })
    };

    barrier.wait();
    let started = Instant::now();
    let reader_latencies = collect_reader_latencies(reader_handles)?;
    let writer_latencies = join_worker(writer)?;
    let duration_ms = started.elapsed().as_secs_f64() * 1000.0;
    let final_seq = shared
        .read()
        .map_err(|error| error.to_string())?
        .storage_stats()
        .map_err(|error| error.to_string())?
        .current_seq
        .0;
    Ok(curve_point(
        readers,
        duration_ms,
        reader_latencies,
        writer_latencies,
        final_seq,
    ))
}

fn run_rwlock_scenario(args: &Args, db_path: &Path, readers: usize) -> Result<Value, String> {
    let db = seeded_database(db_path, args.cells)?;
    let shared = Arc::new(WriterPrefRwLock::new(db));
    let barrier = Arc::new(Barrier::new(readers + 2));
    let mut reader_handles = Vec::new();
    for _ in 0..readers {
        let shared = Arc::clone(&shared);
        let barrier = Arc::clone(&barrier);
        let read_ops = args.read_ops_per_thread;
        let view = bench_view();
        reader_handles.push(thread::spawn(move || {
            barrier.wait();
            let mut latencies = Vec::with_capacity(read_ops);
            for _ in 0..read_ops {
                let started = Instant::now();
                let guard = shared.read();
                let cells = guard
                    .retrieve_aql(QUERY, &view)
                    .map_err(|error| error.to_string())?;
                if cells.is_empty() {
                    return Err("direct retrieve returned no cells".to_owned());
                }
                latencies.push(started.elapsed().as_secs_f64() * 1000.0);
            }
            Ok::<_, String>(latencies)
        }));
    }
    let writer = {
        let shared = Arc::clone(&shared);
        let barrier = Arc::clone(&barrier);
        let cells = args.cells;
        let writer_ops = args.writer_ops;
        thread::spawn(move || {
            barrier.wait();
            let mut latencies = Vec::with_capacity(writer_ops);
            for offset in 0..writer_ops {
                let cell_id = cells + offset + 1;
                let started = Instant::now();
                let mut guard = shared.write();
                guard
                    .put_cell(CellId(cell_id as u64), payload(cell_id))
                    .map_err(|error| error.to_string())?;
                latencies.push(started.elapsed().as_secs_f64() * 1000.0);
            }
            Ok::<_, String>(latencies)
        })
    };

    barrier.wait();
    let started = Instant::now();
    let reader_latencies = collect_reader_latencies(reader_handles)?;
    let writer_latencies = join_worker(writer)?;
    let duration_ms = started.elapsed().as_secs_f64() * 1000.0;
    let final_seq = shared
        .read()
        .storage_stats()
        .map_err(|error| error.to_string())?
        .current_seq
        .0;
    Ok(curve_point(
        readers,
        duration_ms,
        reader_latencies,
        writer_latencies,
        final_seq,
    ))
}

fn seeded_database(db_path: &Path, cells: usize) -> Result<Database, String> {
    fs::remove_dir_all(db_path).ok();
    let mut db = Database::open(db_path).map_err(|error| error.to_string())?;
    let payloads = (1..=cells)
        .map(|index| (CellId(index as u64), payload(index)))
        .collect::<Vec<_>>();
    db.put_cells(payloads).map_err(|error| error.to_string())?;
    Ok(db)
}

fn collect_reader_latencies(
    handles: Vec<thread::JoinHandle<Result<Vec<f64>, String>>>,
) -> Result<Vec<f64>, String> {
    let mut latencies = Vec::new();
    for handle in handles {
        latencies.extend(join_worker(handle)?);
    }
    Ok(latencies)
}

fn join_worker(handle: thread::JoinHandle<Result<Vec<f64>, String>>) -> Result<Vec<f64>, String> {
    match handle.join() {
        Ok(result) => result,
        Err(_) => Err("benchmark worker panicked".to_owned()),
    }
}

fn curve_point(
    readers: usize,
    duration_ms: f64,
    reader_latencies: Vec<f64>,
    writer_latencies: Vec<f64>,
    final_seq: u64,
) -> Value {
    let read_operations = reader_latencies.len();
    let write_operations = writer_latencies.len();
    let read_throughput = throughput(read_operations, duration_ms);
    let write_throughput = throughput(write_operations, duration_ms);
    json!({
        "reader_threads": readers,
        "read_operations": read_operations,
        "write_operations": write_operations,
        "duration_ms": round_ms(duration_ms),
        "read_throughput_per_sec": round_ms(read_throughput),
        "write_throughput_per_sec": round_ms(write_throughput),
        "total_throughput_per_sec": round_ms(read_throughput + write_throughput),
        "reader_latency": latency_summary(&reader_latencies),
        "writer_latency": latency_summary(&writer_latencies),
        "final_seq": final_seq,
    })
}

fn throughput(operations: usize, duration_ms: f64) -> f64 {
    if duration_ms <= 0.0 {
        0.0
    } else {
        operations as f64 / (duration_ms / 1000.0)
    }
}

fn unique_id() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0)
}
