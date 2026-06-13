use std::path::PathBuf;

use cortex_engine::PayloadResidency;

pub(super) struct Args {
    pub(super) root: PathBuf,
    pub(super) report: PathBuf,
    pub(super) cells: usize,
    pub(super) batch_size: usize,
    pub(super) direct_checkpoint: bool,
    pub(super) payload_bytes: usize,
    pub(super) reopen_only: bool,
    pub(super) read_samples: usize,
    pub(super) max_rss_to_estimated_total_ratio: f64,
    pub(super) payload_residency: PayloadResidency,
}

impl Args {
    pub(super) fn parse(values: impl Iterator<Item = String>) -> Result<Self, String> {
        let mut args = Self {
            root: PathBuf::from("target/memory-profile"),
            report: PathBuf::from("target/memory-profile/report.json"),
            cells: 10_000,
            batch_size: 5_000,
            direct_checkpoint: false,
            payload_bytes: 0,
            reopen_only: false,
            read_samples: 0,
            max_rss_to_estimated_total_ratio: 128.0,
            payload_residency: PayloadResidency::Memory,
        };
        let mut values = values.peekable();
        while let Some(arg) = values.next() {
            match arg.as_str() {
                "--root" => args.root = PathBuf::from(next_value(&mut values, "--root")?),
                "--report" => args.report = PathBuf::from(next_value(&mut values, "--report")?),
                "--cells" => {
                    args.cells = parse_usize(next_value(&mut values, "--cells")?, "--cells")?
                }
                "--batch-size" => {
                    args.batch_size =
                        parse_usize(next_value(&mut values, "--batch-size")?, "--batch-size")?
                }
                "--direct-checkpoint" => args.direct_checkpoint = true,
                "--payload-bytes" => {
                    args.payload_bytes = parse_usize(
                        next_value(&mut values, "--payload-bytes")?,
                        "--payload-bytes",
                    )?
                }
                "--reopen-only" => args.reopen_only = true,
                "--read-samples" => {
                    args.read_samples =
                        parse_usize(next_value(&mut values, "--read-samples")?, "--read-samples")?
                }
                "--max-rss-to-estimated-total-ratio" => {
                    args.max_rss_to_estimated_total_ratio = parse_f64(
                        next_value(&mut values, "--max-rss-to-estimated-total-ratio")?,
                        "--max-rss-to-estimated-total-ratio",
                    )?
                }
                "--payload-residency" => {
                    args.payload_residency =
                        parse_payload_residency(next_value(&mut values, "--payload-residency")?)?
                }
                "--help" | "-h" => return Err(help_text()),
                unknown => return Err(format!("unknown argument: {unknown}\n{}", help_text())),
            }
        }
        if args.cells == 0 {
            return Err("--cells must be positive".to_owned());
        }
        if args.batch_size == 0 {
            return Err("--batch-size must be positive".to_owned());
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

fn parse_payload_residency(value: String) -> Result<PayloadResidency, String> {
    match value.trim().to_ascii_lowercase().as_str() {
        "memory" => Ok(PayloadResidency::Memory),
        "lazy" => Ok(PayloadResidency::Lazy),
        _ => Err(format!(
            "invalid value for --payload-residency: {value}; expected memory or lazy"
        )),
    }
}

fn help_text() -> String {
    "usage: memory_profile_check [--root PATH] [--report PATH] [--cells N] [--batch-size N] [--direct-checkpoint] [--payload-bytes N] [--reopen-only] [--read-samples N] [--payload-residency memory|lazy] [--max-rss-to-estimated-total-ratio N]".to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(values: &[&str]) -> Args {
        Args::parse(values.iter().map(|value| (*value).to_owned())).unwrap()
    }

    #[test]
    fn parse_payload_bytes_override() {
        let args = parse(&["--cells", "12", "--payload-bytes", "4096"]);

        assert_eq!(args.cells, 12);
        assert_eq!(args.payload_bytes, 4096);
    }

    #[test]
    fn parse_batch_size_override() {
        let args = parse(&["--batch-size", "77"]);

        assert_eq!(args.batch_size, 77);
    }

    #[test]
    fn parse_direct_checkpoint_flag() {
        let args = parse(&["--direct-checkpoint"]);

        assert!(args.direct_checkpoint);
    }

    #[test]
    fn parse_keeps_payload_bytes_disabled_by_default() {
        let args = parse(&[]);

        assert_eq!(args.payload_bytes, 0);
    }

    #[test]
    fn parse_reopen_only_flag() {
        let args = parse(&["--reopen-only"]);

        assert!(args.reopen_only);
    }

    #[test]
    fn parse_read_samples_override() {
        let args = parse(&["--read-samples", "25"]);

        assert_eq!(args.read_samples, 25);
    }

    #[test]
    fn parse_rejects_zero_batch_size() {
        let error = match Args::parse(["--batch-size", "0"].into_iter().map(str::to_owned)) {
            Ok(_) => panic!("expected zero batch size to fail"),
            Err(error) => error,
        };

        assert_eq!(error, "--batch-size must be positive");
    }
}
