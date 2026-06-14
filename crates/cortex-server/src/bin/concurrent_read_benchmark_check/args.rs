use std::path::{Path, PathBuf};

#[derive(Debug, PartialEq)]
pub(super) struct Args {
    pub(super) root: PathBuf,
    pub(super) report: PathBuf,
    pub(super) markdown: PathBuf,
    pub(super) cells: usize,
    pub(super) reader_threads: Vec<usize>,
    pub(super) read_ops_per_thread: usize,
    pub(super) writer_ops: usize,
    pub(super) max_p95_ms: f64,
    pub(super) self_test: bool,
}

impl Args {
    pub(super) fn parse(values: impl IntoIterator<Item = String>) -> Result<Self, String> {
        let mut parsed = Self {
            root: PathBuf::from("target/concurrent-read-benchmark"),
            report: PathBuf::from("target/concurrent-read-benchmark/report.json"),
            markdown: PathBuf::from("target/concurrent-read-benchmark/report.md"),
            cells: 200,
            reader_threads: vec![1, 2, 4],
            read_ops_per_thread: 10,
            writer_ops: 5,
            max_p95_ms: 1000.0,
            self_test: false,
        };
        let mut args = values.into_iter();
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--root" => parsed.root = PathBuf::from(next_value(&mut args, &arg)?),
                "--report" => parsed.report = PathBuf::from(next_value(&mut args, &arg)?),
                "--markdown" => parsed.markdown = PathBuf::from(next_value(&mut args, &arg)?),
                "--cells" => parsed.cells = parse_positive(&next_value(&mut args, &arg)?, &arg)?,
                "--reader-threads" => {
                    parsed.reader_threads = parse_thread_counts(&next_value(&mut args, &arg)?)?
                }
                "--read-ops-per-thread" => {
                    parsed.read_ops_per_thread =
                        parse_positive(&next_value(&mut args, &arg)?, &arg)?
                }
                "--writer-ops" => {
                    parsed.writer_ops = parse_positive(&next_value(&mut args, &arg)?, &arg)?
                }
                "--max-p95-ms" => {
                    parsed.max_p95_ms = parse_f64(&next_value(&mut args, &arg)?, &arg)?
                }
                "--self-test" => parsed.self_test = true,
                "--help" | "-h" => return Err(usage()),
                value => return Err(format!("unknown option: {value}")),
            }
        }
        Ok(parsed)
    }
}

pub(super) fn self_test() -> Result<(), String> {
    let args = Args::parse([
        "--root".to_owned(),
        "target/x".to_owned(),
        "--report".to_owned(),
        "target/x/report.json".to_owned(),
        "--markdown".to_owned(),
        "target/x/report.md".to_owned(),
        "--cells".to_owned(),
        "4".to_owned(),
        "--reader-threads".to_owned(),
        "1,3".to_owned(),
        "--read-ops-per-thread".to_owned(),
        "2".to_owned(),
        "--writer-ops".to_owned(),
        "1".to_owned(),
        "--max-p95-ms".to_owned(),
        "50.5".to_owned(),
    ])?;
    if args.root != Path::new("target/x")
        || args.report != Path::new("target/x/report.json")
        || args.markdown != Path::new("target/x/report.md")
        || args.cells != 4
        || args.reader_threads != vec![1, 3]
        || args.read_ops_per_thread != 2
        || args.writer_ops != 1
        || args.max_p95_ms != 50.5
    {
        return Err("argument parser self-test failed".to_owned());
    }
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

fn parse_thread_counts(value: &str) -> Result<Vec<usize>, String> {
    let counts = value
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(|item| parse_positive(item, "--reader-threads"))
        .collect::<Result<Vec<_>, _>>()?;
    if counts.is_empty() {
        return Err("--reader-threads must include at least one value".to_owned());
    }
    Ok(counts)
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

fn usage() -> String {
    concat!(
        "usage: concurrent_read_benchmark_check [--root PATH] [--report PATH] ",
        "[--markdown PATH] [--cells N] [--reader-threads CSV] ",
        "[--read-ops-per-thread N] [--writer-ops N] [--max-p95-ms N]"
    )
    .to_owned()
}
