use std::path::{Path, PathBuf};

#[derive(Debug, PartialEq)]
pub struct Args {
    pub root: PathBuf,
    pub report: PathBuf,
    pub cells: usize,
    pub max_total_ms: f64,
    pub min_ingest_cells_per_sec: f64,
    pub max_rss_bytes: u64,
    pub self_test: bool,
}

impl Args {
    pub fn parse(args: impl IntoIterator<Item = String>) -> Result<Self, String> {
        let mut parsed = Self {
            root: PathBuf::from("target/single-node-performance"),
            report: PathBuf::from("target/single-node-performance/report.json"),
            cells: 500,
            max_total_ms: 30_000.0,
            min_ingest_cells_per_sec: 1.0,
            max_rss_bytes: 1_073_741_824,
            self_test: false,
        };
        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--root" => parsed.root = PathBuf::from(next_value(&mut args, &arg)?),
                "--report" => parsed.report = PathBuf::from(next_value(&mut args, &arg)?),
                "--cells" => parsed.cells = parse_positive(&next_value(&mut args, &arg)?, &arg)?,
                "--max-total-ms" => {
                    parsed.max_total_ms = parse_f64(&next_value(&mut args, &arg)?, &arg)?
                }
                "--min-ingest-cells-per-sec" => {
                    parsed.min_ingest_cells_per_sec =
                        parse_f64(&next_value(&mut args, &arg)?, &arg)?
                }
                "--max-rss-bytes" => {
                    parsed.max_rss_bytes = parse_u64(&next_value(&mut args, &arg)?, &arg)?
                }
                "--self-test" => parsed.self_test = true,
                "--help" | "-h" => return Err(usage()),
                value => return Err(format!("unknown option: {value}")),
            }
        }
        Ok(parsed)
    }
}

pub fn self_test() -> Result<(), String> {
    let args = Args::parse([
        "--root".to_owned(),
        "target/x".to_owned(),
        "--report".to_owned(),
        "target/x/report.json".to_owned(),
        "--cells".to_owned(),
        "4".to_owned(),
        "--min-ingest-cells-per-sec".to_owned(),
        "1.5".to_owned(),
        "--max-rss-bytes".to_owned(),
        "2048".to_owned(),
    ])?;
    if args.cells != 4
        || args.root != Path::new("target/x")
        || args.min_ingest_cells_per_sec != 1.5
        || args.max_rss_bytes != 2048
    {
        return Err("argument parser self-test failed".to_owned());
    }
    println!("single-node performance self-test passed");
    Ok(())
}

fn next_value(args: &mut impl Iterator<Item = String>, option: &str) -> Result<String, String> {
    args.next()
        .ok_or_else(|| format!("{option} requires a value"))
}

fn parse_positive(value: &str, option: &str) -> Result<usize, String> {
    let parsed = value
        .parse::<usize>()
        .map_err(|error| format!("{option} must be usize: {error}"))?;
    if parsed == 0 {
        return Err(format!("{option} must be greater than zero"));
    }
    Ok(parsed)
}

fn parse_f64(value: &str, option: &str) -> Result<f64, String> {
    let parsed = value
        .parse::<f64>()
        .map_err(|error| format!("{option} must be number: {error}"))?;
    if parsed < 0.0 {
        return Err(format!("{option} must be non-negative"));
    }
    Ok(parsed)
}

fn parse_u64(value: &str, option: &str) -> Result<u64, String> {
    value
        .parse::<u64>()
        .map_err(|error| format!("{option} must be u64: {error}"))
}

fn usage() -> String {
    concat!(
        "usage: single_node_performance_check [--root PATH] [--report PATH] ",
        "[--cells N] [--max-total-ms N] [--min-ingest-cells-per-sec N] ",
        "[--max-rss-bytes N]"
    )
    .to_owned()
}
