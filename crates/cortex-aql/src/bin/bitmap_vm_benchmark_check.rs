use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::Instant;

use cortex_aql::{
    compute_bitmap_stack_depth, eval_bitmap_program_bitmap, BitmapHandle, BitmapOp, BitmapProgram,
    BitmapProvider, RoaringBitmap,
};

const DEFAULT_CANDIDATES: u32 = 1_000_000;
const DEFAULT_ITERATIONS: u32 = 20;

#[derive(Clone)]
struct BenchProvider {
    bitmaps: BTreeMap<BitmapHandle, RoaringBitmap>,
    universe: RoaringBitmap,
}

impl BitmapProvider for BenchProvider {
    fn bitmap(&self, handle: BitmapHandle) -> Option<RoaringBitmap> {
        self.bitmaps.get(&handle).cloned()
    }

    fn agent_allowed(&self) -> RoaringBitmap {
        self.universe.clone()
    }

    fn live(&self) -> RoaringBitmap {
        self.universe.clone()
    }

    fn universe(&self) -> RoaringBitmap {
        self.universe.clone()
    }
}

#[derive(Clone, Debug)]
struct Args {
    candidates: u32,
    iterations: u32,
    max_min_ms: Option<f64>,
    report: PathBuf,
}

#[derive(Clone, Debug)]
struct BenchResult {
    op: &'static str,
    expected_len: u64,
    min_ms: f64,
    avg_ms: f64,
}

#[derive(Clone, Debug)]
struct Footprint {
    legacy_acb_bytes: u64,
    roaring_acb_bytes: u64,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let args = parse_args(std::env::args().skip(1))?;
    let provider = provider(args.candidates);
    let footprint = footprint(&provider);
    let results = vec![
        run_case(
            "and",
            program(vec![
                BitmapOp::Push(BitmapHandle(2)),
                BitmapOp::Push(BitmapHandle(3)),
                BitmapOp::And,
            ])?,
            &provider,
            count_multiples_below(args.candidates, 6),
            args.iterations,
        )?,
        run_case(
            "or",
            program(vec![
                BitmapOp::Push(BitmapHandle(2)),
                BitmapOp::Push(BitmapHandle(3)),
                BitmapOp::Or,
            ])?,
            &provider,
            count_multiples_below(args.candidates, 2) + count_multiples_below(args.candidates, 3)
                - count_multiples_below(args.candidates, 6),
            args.iterations,
        )?,
        run_case(
            "not",
            program(vec![BitmapOp::Push(BitmapHandle(2)), BitmapOp::Not])?,
            &provider,
            u64::from(args.candidates) - count_multiples_below(args.candidates, 2),
            args.iterations,
        )?,
    ];

    if let Some(max_min_ms) = args.max_min_ms {
        for result in &results {
            if result.min_ms > max_min_ms {
                return Err(format!(
                    "bitmap {} min_ms {:.3} exceeded {:.3}",
                    result.op, result.min_ms, max_min_ms
                ));
            }
        }
    }

    write_report(&args, &results, &footprint)?;
    println!(
        "bitmap vm benchmark passed: candidates={} report={}",
        args.candidates,
        args.report.display()
    );
    Ok(())
}

fn parse_args(input: impl IntoIterator<Item = String>) -> Result<Args, String> {
    let mut args = Args {
        candidates: DEFAULT_CANDIDATES,
        iterations: DEFAULT_ITERATIONS,
        max_min_ms: None,
        report: PathBuf::from("target/bitmap-vm-benchmark/report.json"),
    };
    let mut iter = input.into_iter();
    while let Some(flag) = iter.next() {
        match flag.as_str() {
            "--candidates" => {
                args.candidates = parse_u32(&flag, iter.next())?;
            }
            "--iterations" => {
                args.iterations = parse_u32(&flag, iter.next())?;
            }
            "--max-min-ms" => {
                args.max_min_ms = Some(parse_f64(&flag, iter.next())?);
            }
            "--report" => {
                args.report = PathBuf::from(value_for(&flag, iter.next())?);
            }
            "--help" | "-h" => {
                return Err(
                    "usage: bitmap_vm_benchmark_check [--candidates N] [--iterations N] [--max-min-ms F] [--report PATH]"
                        .to_owned(),
                );
            }
            _ => return Err(format!("unknown argument: {flag}")),
        }
    }
    if args.candidates == 0 {
        return Err("--candidates must be > 0".to_owned());
    }
    if args.iterations == 0 {
        return Err("--iterations must be > 0".to_owned());
    }
    Ok(args)
}

fn value_for(flag: &str, value: Option<String>) -> Result<String, String> {
    value.ok_or_else(|| format!("{flag} requires a value"))
}

fn parse_u32(flag: &str, value: Option<String>) -> Result<u32, String> {
    value_for(flag, value)?
        .parse::<u32>()
        .map_err(|error| format!("{flag}: invalid integer: {error}"))
}

fn parse_f64(flag: &str, value: Option<String>) -> Result<f64, String> {
    let parsed = value_for(flag, value)?
        .parse::<f64>()
        .map_err(|error| format!("{flag}: invalid number: {error}"))?;
    if parsed.is_sign_positive() {
        Ok(parsed)
    } else {
        Err(format!("{flag}: value must be positive"))
    }
}

fn provider(candidates: u32) -> BenchProvider {
    BenchProvider {
        bitmaps: BTreeMap::from([
            (BitmapHandle(2), roaring((0..candidates).step_by(2))),
            (BitmapHandle(3), roaring((0..candidates).step_by(3))),
        ]),
        universe: roaring(0..candidates),
    }
}

fn roaring(values: impl IntoIterator<Item = u32>) -> RoaringBitmap {
    values.into_iter().collect()
}

fn program(ops: Vec<BitmapOp>) -> Result<BitmapProgram, String> {
    let max_stack_depth = compute_bitmap_stack_depth(&ops)
        .map_err(|error| format!("invalid bitmap program: {error}"))?;
    Ok(BitmapProgram {
        ops,
        max_stack_depth,
    })
}

fn run_case(
    op: &'static str,
    program: BitmapProgram,
    provider: &BenchProvider,
    expected_len: u64,
    iterations: u32,
) -> Result<BenchResult, String> {
    let mut total_ms = 0.0;
    let mut min_ms = f64::MAX;
    for _ in 0..iterations {
        let started = Instant::now();
        let result = eval_bitmap_program_bitmap(&program, provider)
            .map_err(|error| format!("{op}: bitmap VM failed: {error}"))?;
        let elapsed_ms = started.elapsed().as_secs_f64() * 1000.0;
        if result.len() != expected_len {
            return Err(format!(
                "{op}: expected len {expected_len}, got {}",
                result.len()
            ));
        }
        total_ms += elapsed_ms;
        min_ms = min_ms.min(elapsed_ms);
    }
    Ok(BenchResult {
        op,
        expected_len,
        min_ms,
        avg_ms: total_ms / f64::from(iterations),
    })
}

fn count_multiples_below(candidates: u32, step: u32) -> u64 {
    if candidates == 0 {
        0
    } else {
        u64::from(((candidates - 1) / step) + 1)
    }
}

fn footprint(provider: &BenchProvider) -> Footprint {
    let legacy_acb_bytes = 4
        + 4
        + provider
            .bitmaps
            .values()
            .map(|bitmap| 8 + 4 + bitmap.len() * 4)
            .sum::<u64>()
        + 4;
    let roaring_acb_bytes = 4
        + 4
        + provider
            .bitmaps
            .values()
            .map(|bitmap| 8 + 4 + bitmap.serialized_size() as u64)
            .sum::<u64>()
        + 4;
    Footprint {
        legacy_acb_bytes,
        roaring_acb_bytes,
    }
}

fn write_report(args: &Args, results: &[BenchResult], footprint: &Footprint) -> Result<(), String> {
    if let Some(parent) = args.report.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create report dir: {error}"))?;
    }
    let rows = results
        .iter()
        .map(|result| {
            format!(
                "{{\"op\":\"{}\",\"expected_len\":{},\"min_ms\":{:.6},\"avg_ms\":{:.6}}}",
                result.op, result.expected_len, result.min_ms, result.avg_ms
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    let body = format!(
        "{{\"schema_version\":\"cortexdb.bitmap_vm_benchmark.v1\",\"candidates\":{},\"iterations\":{},\"legacy_acb_bytes\":{},\"roaring_acb_bytes\":{},\"storage_reduction_pct\":{:.3},\"results\":[{}]}}\n",
        args.candidates,
        args.iterations,
        footprint.legacy_acb_bytes,
        footprint.roaring_acb_bytes,
        storage_reduction_pct(footprint),
        rows
    );
    std::fs::write(&args.report, body).map_err(|error| format!("failed to write report: {error}"))
}

fn storage_reduction_pct(footprint: &Footprint) -> f64 {
    if footprint.legacy_acb_bytes == 0 {
        0.0
    } else {
        let saved = footprint
            .legacy_acb_bytes
            .saturating_sub(footprint.roaring_acb_bytes);
        (saved as f64) * 100.0 / (footprint.legacy_acb_bytes as f64)
    }
}
