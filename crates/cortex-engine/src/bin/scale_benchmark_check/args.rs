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
    pub(crate) direct_checkpoint: bool,
    pub(crate) reopen_only: bool,
    pub(crate) payload_residency: cortex_engine::PayloadResidency,
    pub(crate) skip_storage_estimates: bool,
    pub(crate) skip_validation: bool,
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
            direct_checkpoint: false,
            reopen_only: false,
            payload_residency: cortex_engine::PayloadResidency::Memory,
            skip_storage_estimates: false,
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
                "--direct-checkpoint" => args.direct_checkpoint = true,
                "--reopen-only" => args.reopen_only = true,
                "--skip-storage-estimates" => args.skip_storage_estimates = true,
                "--skip-validation" => args.skip_validation = true,
                "--payload-residency" => {
                    args.payload_residency =
                        residency::parse(next_value(&mut values, "--payload-residency")?)?
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
    "usage: scale_benchmark_check [--root PATH] [--report PATH] [--cells N] [--samples N] [--search-samples N] [--context-samples N] [--verify-samples N] [--batch-size N] [--payload-bytes N] [--direct-checkpoint] [--reopen-only] [--skip-storage-estimates] [--skip-validation] [--payload-residency memory|lazy]".to_owned()
}

#[path = "residency.rs"]
mod residency;

#[cfg(test)]
#[path = "args_tests.rs"]
mod tests;
