use std::path::PathBuf;

use cortex_engine::PayloadResidency;

pub(super) struct Args {
    pub(super) root: PathBuf,
    pub(super) report: PathBuf,
    pub(super) cells: usize,
    pub(super) max_rss_to_estimated_total_ratio: f64,
    pub(super) payload_residency: PayloadResidency,
}

impl Args {
    pub(super) fn parse(values: impl Iterator<Item = String>) -> Result<Self, String> {
        let mut args = Self {
            root: PathBuf::from("target/memory-profile"),
            report: PathBuf::from("target/memory-profile/report.json"),
            cells: 10_000,
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
    "usage: memory_profile_check [--root PATH] [--report PATH] [--cells N] [--payload-residency memory|lazy] [--max-rss-to-estimated-total-ratio N]".to_owned()
}
