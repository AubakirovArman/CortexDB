use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq)]
pub struct Args {
    pub root: PathBuf,
    pub report: PathBuf,
    pub markdown: Option<PathBuf>,
    pub docs: usize,
    pub ingestion_batch_size: usize,
    pub embedding_batch_size: usize,
    pub payload_bytes: usize,
    pub resume_after_docs: Option<usize>,
    pub min_docs_per_sec: f64,
    pub self_test: bool,
}

impl Args {
    pub fn parse(args: impl IntoIterator<Item = String>) -> Result<Self, String> {
        let mut parsed = Self {
            root: PathBuf::from("target/ingestion-throughput"),
            report: PathBuf::from("target/ingestion-throughput/report.json"),
            markdown: Some(PathBuf::from("target/ingestion-throughput/report.md")),
            docs: 100_000,
            ingestion_batch_size: 1_000,
            embedding_batch_size: 128,
            payload_bytes: 256,
            resume_after_docs: None,
            min_docs_per_sec: 1.0,
            self_test: false,
        };
        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--root" => parsed.root = PathBuf::from(next_value(&mut args, &arg)?),
                "--report" => parsed.report = PathBuf::from(next_value(&mut args, &arg)?),
                "--markdown" => parsed.markdown = Some(PathBuf::from(next_value(&mut args, &arg)?)),
                "--no-markdown" => parsed.markdown = None,
                "--docs" => parsed.docs = parse_positive(&next_value(&mut args, &arg)?, &arg)?,
                "--ingestion-batch-size" => {
                    parsed.ingestion_batch_size =
                        parse_positive(&next_value(&mut args, &arg)?, &arg)?
                }
                "--embedding-batch-size" => {
                    parsed.embedding_batch_size =
                        parse_positive(&next_value(&mut args, &arg)?, &arg)?
                }
                "--payload-bytes" => {
                    parsed.payload_bytes = parse_positive(&next_value(&mut args, &arg)?, &arg)?
                }
                "--resume-after-docs" => {
                    parsed.resume_after_docs =
                        Some(parse_positive(&next_value(&mut args, &arg)?, &arg)?)
                }
                "--min-docs-per-sec" => {
                    parsed.min_docs_per_sec = parse_f64(&next_value(&mut args, &arg)?, &arg)?
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
        "target/c19".to_owned(),
        "--report".to_owned(),
        "target/c19/report.json".to_owned(),
        "--docs".to_owned(),
        "42".to_owned(),
        "--ingestion-batch-size".to_owned(),
        "7".to_owned(),
        "--embedding-batch-size".to_owned(),
        "3".to_owned(),
        "--payload-bytes".to_owned(),
        "64".to_owned(),
        "--resume-after-docs".to_owned(),
        "11".to_owned(),
    ])?;
    if args.root != Path::new("target/c19")
        || args.report != Path::new("target/c19/report.json")
        || args.docs != 42
        || args.ingestion_batch_size != 7
        || args.embedding_batch_size != 3
        || args.payload_bytes != 64
        || args.resume_after_docs != Some(11)
    {
        return Err("ingestion throughput args self-test failed".to_owned());
    }
    println!("ingestion throughput args self-test passed");
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

fn usage() -> String {
    concat!(
        "usage: ingestion_throughput_check [--root PATH] [--report PATH] ",
        "[--markdown PATH] [--no-markdown] [--docs N] ",
        "[--ingestion-batch-size N] [--embedding-batch-size N] ",
        "[--payload-bytes N] [--resume-after-docs N] [--min-docs-per-sec N]"
    )
    .to_owned()
}
