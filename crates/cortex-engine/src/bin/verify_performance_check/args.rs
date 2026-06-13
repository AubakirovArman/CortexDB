use std::path::PathBuf;

pub(super) struct Args {
    pub(super) root: PathBuf,
    pub(super) report: PathBuf,
    pub(super) cells: usize,
    pub(super) warmup_samples: usize,
    pub(super) samples: usize,
    pub(super) max_p95_ms: f64,
}

impl Args {
    pub(super) fn parse(values: impl Iterator<Item = String>) -> Result<Self, String> {
        let mut args = Self {
            root: PathBuf::from("target/verify-performance"),
            report: PathBuf::from("target/verify-performance/report.json"),
            cells: 10_000,
            warmup_samples: 1,
            samples: 25,
            max_p95_ms: 250.0,
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
                "--warmup-samples" => {
                    args.warmup_samples = parse_usize(
                        next_value(&mut values, "--warmup-samples")?,
                        "--warmup-samples",
                    )?
                }
                "--max-p95-ms" => {
                    args.max_p95_ms =
                        parse_f64(next_value(&mut values, "--max-p95-ms")?, "--max-p95-ms")?
                }
                "--help" | "-h" => return Err(help_text()),
                unknown => return Err(format!("unknown argument: {unknown}\n{}", help_text())),
            }
        }
        if args.cells == 0 {
            return Err("--cells must be positive".to_owned());
        }
        if args.samples == 0 {
            return Err("--samples must be positive".to_owned());
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

fn parse_f64(value: String, flag: &str) -> Result<f64, String> {
    value
        .parse::<f64>()
        .map_err(|_| format!("invalid value for {flag}: {value}"))
}

fn help_text() -> String {
    "usage: verify_performance_check [--root PATH] [--report PATH] [--cells N] [--warmup-samples N] [--samples N] [--max-p95-ms MS]".to_owned()
}
