use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Args {
    pub(super) questions: PathBuf,
    pub(super) output: PathBuf,
    pub(super) limit: Option<usize>,
    pub(super) offset: usize,
    pub(super) min_multi_coverage_pct: u64,
}

pub(super) fn parse_args(mut args: impl Iterator<Item = String>) -> Result<Args, String> {
    let mut questions = None;
    let mut output = None;
    let mut limit = None;
    let mut offset = 0usize;
    let mut min_multi_coverage_pct = 80u64;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--questions" | "--questions-file" => {
                questions = Some(PathBuf::from(next_value(&mut args, &arg)?));
            }
            "--output" | "--report" => output = Some(PathBuf::from(next_value(&mut args, &arg)?)),
            "--limit" => limit = Some(parse_usize(&next_value(&mut args, &arg)?, &arg)?),
            "--offset" => offset = parse_usize(&next_value(&mut args, &arg)?, &arg)?,
            "--min-multi-coverage-pct" => {
                min_multi_coverage_pct = parse_u64(&next_value(&mut args, &arg)?, &arg)?.min(100)
            }
            "--help" | "-h" => return Err(usage()),
            _ => return Err(format!("unknown argument {arg}\n{}", usage())),
        }
    }
    Ok(Args {
        questions: questions.ok_or_else(usage)?,
        output: output.ok_or_else(usage)?,
        limit,
        offset,
        min_multi_coverage_pct,
    })
}

fn next_value(args: &mut impl Iterator<Item = String>, name: &str) -> Result<String, String> {
    args.next()
        .ok_or_else(|| format!("{name} requires a value\n{}", usage()))
}

fn parse_usize(value: &str, name: &str) -> Result<usize, String> {
    value
        .parse::<usize>()
        .map_err(|error| format!("{name} expects a positive integer: {error}"))
}

fn parse_u64(value: &str, name: &str) -> Result<u64, String> {
    value
        .parse::<u64>()
        .map_err(|error| format!("{name} expects an integer: {error}"))
}

fn usage() -> String {
    "usage: enterprise_rag_decomposition_check --questions <questions.jsonl> --output <report.json> [--limit N] [--offset N] [--min-multi-coverage-pct PCT]".to_owned()
}

#[cfg(test)]
mod tests {
    use super::parse_args;

    #[test]
    fn parse_args_accepts_named_paths_and_threshold() {
        let args = parse_args(
            [
                "--questions",
                "questions.jsonl",
                "--output",
                "report.json",
                "--limit",
                "50",
                "--min-multi-coverage-pct",
                "80",
            ]
            .into_iter()
            .map(str::to_owned),
        )
        .unwrap();

        assert_eq!(args.limit, Some(50));
        assert_eq!(args.min_multi_coverage_pct, 80);
    }
}
