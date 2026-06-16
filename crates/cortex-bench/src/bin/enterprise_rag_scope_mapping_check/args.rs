use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Args {
    pub(crate) questions: PathBuf,
    pub(crate) output: PathBuf,
    pub(crate) limit: Option<usize>,
    pub(crate) offset: usize,
    pub(crate) min_project_related_coverage_pct: u64,
}

pub(crate) fn parse_args(mut args: impl Iterator<Item = String>) -> Result<Args, String> {
    let mut questions = None;
    let mut output = None;
    let mut limit = None;
    let mut offset = 0usize;
    let mut min_project_related_coverage_pct = 70u64;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--questions" | "--questions-file" => {
                questions = Some(PathBuf::from(next_value(&mut args, &arg)?));
            }
            "--output" | "--report" => output = Some(PathBuf::from(next_value(&mut args, &arg)?)),
            "--limit" => limit = Some(parse_usize(&next_value(&mut args, &arg)?, &arg)?),
            "--offset" => offset = parse_usize(&next_value(&mut args, &arg)?, &arg)?,
            "--min-project-related-coverage-pct" => {
                min_project_related_coverage_pct =
                    parse_u64(&next_value(&mut args, &arg)?, &arg)?.min(100)
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
        min_project_related_coverage_pct,
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
    "usage: enterprise_rag_scope_mapping_check --questions <questions.jsonl> --output <report.json> [--limit N] [--offset N] [--min-project-related-coverage-pct PCT]".to_owned()
}
