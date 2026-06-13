use std::path::PathBuf;

#[derive(Debug, PartialEq, Eq)]
pub(super) struct Args {
    pub(super) documents: PathBuf,
    pub(super) questions: PathBuf,
    pub(super) output: Option<PathBuf>,
    pub(super) top_k: usize,
    pub(super) min_average_recall_delta_pct: i64,
    pub(super) min_full_recall_delta: i64,
    pub(super) min_engine_average_recall_pct: u64,
}

impl Args {
    pub(super) fn parse(args: impl IntoIterator<Item = String>) -> Result<Self, String> {
        let mut documents = None;
        let mut questions = None;
        let mut output = None;
        let mut top_k = 3usize;
        let mut min_average_recall_delta_pct = 20i64;
        let mut min_full_recall_delta = 2i64;
        let mut min_engine_average_recall_pct = 95u64;
        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--documents" => documents = Some(PathBuf::from(next_value(&mut args, &arg)?)),
                "--questions" => questions = Some(PathBuf::from(next_value(&mut args, &arg)?)),
                "--output" => output = Some(PathBuf::from(next_value(&mut args, &arg)?)),
                "--top-k" => top_k = parse_usize(&next_value(&mut args, &arg)?, &arg)?,
                "--min-average-recall-delta-pct" => {
                    min_average_recall_delta_pct = parse_i64(&next_value(&mut args, &arg)?, &arg)?
                }
                "--min-full-recall-delta" => {
                    min_full_recall_delta = parse_i64(&next_value(&mut args, &arg)?, &arg)?
                }
                "--min-engine-average-recall-pct" => {
                    min_engine_average_recall_pct = parse_u64(&next_value(&mut args, &arg)?, &arg)?
                }
                "--help" | "-h" => return Err(usage()),
                value => return Err(format!("unknown option: {value}")),
            }
        }
        Ok(Self {
            documents: documents.ok_or_else(|| "--documents is required".to_owned())?,
            questions: questions.ok_or_else(|| "--questions is required".to_owned())?,
            output,
            top_k,
            min_average_recall_delta_pct,
            min_full_recall_delta,
            min_engine_average_recall_pct,
        })
    }
}

fn next_value(args: &mut impl Iterator<Item = String>, option: &str) -> Result<String, String> {
    args.next()
        .ok_or_else(|| format!("{option} requires a value"))
}

fn parse_usize(value: &str, option: &str) -> Result<usize, String> {
    let parsed = value
        .parse()
        .map_err(|error| format!("{option} must be usize: {error}"))?;
    if parsed == 0 {
        return Err(format!("{option} must be greater than zero"));
    }
    Ok(parsed)
}

fn parse_i64(value: &str, option: &str) -> Result<i64, String> {
    value
        .parse()
        .map_err(|error| format!("{option} must be i64: {error}"))
}

fn parse_u64(value: &str, option: &str) -> Result<u64, String> {
    value
        .parse()
        .map_err(|error| format!("{option} must be u64: {error}"))
}

fn usage() -> String {
    "usage: query_understanding_lift_check --documents PATH --questions PATH \
     [--top-k N] [--output PATH] [--min-average-recall-delta-pct N] \
     [--min-full-recall-delta N] [--min-engine-average-recall-pct N]"
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::Args;

    #[test]
    fn parse_args_rejects_zero_top_k() {
        let error = Args::parse([
            "--documents".to_owned(),
            "docs.jsonl".to_owned(),
            "--questions".to_owned(),
            "questions.jsonl".to_owned(),
            "--top-k".to_owned(),
            "0".to_owned(),
        ])
        .unwrap_err();

        assert!(error.contains("greater than zero"));
    }
}
