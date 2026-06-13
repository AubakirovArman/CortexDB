use std::path::PathBuf;

pub(crate) struct Args {
    pub(crate) root: PathBuf,
    pub(crate) report: PathBuf,
    pub(crate) cells: usize,
    pub(crate) samples: usize,
    pub(crate) search_samples: usize,
    pub(crate) context_samples: usize,
    pub(crate) verify_samples: usize,
    pub(crate) batch_size: usize,
    pub(crate) payload_bytes: Option<usize>,
}

impl Args {
    pub(crate) fn parse(values: impl Iterator<Item = String>) -> Result<Self, String> {
        let mut args = Self {
            root: PathBuf::from("target/scale-bench"),
            report: PathBuf::from("target/scale-bench/report.json"),
            cells: 100_000,
            samples: 100,
            search_samples: 100,
            context_samples: 10,
            verify_samples: 10,
            batch_size: 5_000,
            payload_bytes: None,
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
                "--search-samples" => {
                    args.search_samples = parse_usize(
                        next_value(&mut values, "--search-samples")?,
                        "--search-samples",
                    )?
                }
                "--context-samples" => {
                    args.context_samples = parse_usize(
                        next_value(&mut values, "--context-samples")?,
                        "--context-samples",
                    )?
                }
                "--verify-samples" => {
                    args.verify_samples = parse_usize(
                        next_value(&mut values, "--verify-samples")?,
                        "--verify-samples",
                    )?
                }
                "--batch-size" => {
                    args.batch_size =
                        parse_usize(next_value(&mut values, "--batch-size")?, "--batch-size")?
                }
                "--payload-bytes" => {
                    args.payload_bytes = Some(parse_usize(
                        next_value(&mut values, "--payload-bytes")?,
                        "--payload-bytes",
                    )?)
                }
                "--help" | "-h" => return Err(help_text()),
                unknown => return Err(format!("unknown argument: {unknown}\n{}", help_text())),
            }
        }
        for (name, value) in [
            ("--cells", args.cells),
            ("--batch-size", args.batch_size),
            ("--payload-bytes", args.payload_bytes.unwrap_or(1)),
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

fn help_text() -> String {
    "usage: scale_benchmark_check [--root PATH] [--report PATH] [--cells N] [--samples N] [--search-samples N] [--context-samples N] [--verify-samples N] [--batch-size N] [--payload-bytes N]".to_owned()
}

#[cfg(test)]
mod tests {
    use super::Args;

    #[test]
    fn parse_payload_bytes_override() {
        let args = Args::parse(["--payload-bytes", "128"].into_iter().map(str::to_owned)).unwrap();
        assert_eq!(args.payload_bytes, Some(128));
    }

    #[test]
    fn parse_rejects_zero_payload_bytes() {
        let error = match Args::parse(["--payload-bytes", "0"].into_iter().map(str::to_owned)) {
            Ok(_) => panic!("expected --payload-bytes=0 to fail"),
            Err(error) => error,
        };
        assert!(error.contains("--payload-bytes must be positive"));
    }
}
