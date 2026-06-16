use std::path::PathBuf;

#[derive(Debug, PartialEq, Eq)]
pub struct Args {
    pub data_file: PathBuf,
    pub db_root: PathBuf,
    pub output: PathBuf,
    pub report: Option<PathBuf>,
    pub top_k: usize,
    pub max_questions: Option<usize>,
    pub progress_every: usize,
    pub reset_db: bool,
}

impl Args {
    pub fn parse(args: impl IntoIterator<Item = String>) -> Result<Self, String> {
        let mut data_file = None;
        let mut db_root = None;
        let mut output = None;
        let mut report = None;
        let mut top_k = 10usize;
        let mut max_questions = None;
        let mut progress_every = 250usize;
        let mut reset_db = false;
        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--data-file" => data_file = Some(PathBuf::from(next_value(&mut args, &arg)?)),
                "--db-root" => db_root = Some(PathBuf::from(next_value(&mut args, &arg)?)),
                "--output" => output = Some(PathBuf::from(next_value(&mut args, &arg)?)),
                "--report" => report = Some(PathBuf::from(next_value(&mut args, &arg)?)),
                "--top-k" => top_k = parse_usize(&next_value(&mut args, &arg)?, &arg)?,
                "--max-questions" => {
                    max_questions = Some(parse_usize(&next_value(&mut args, &arg)?, &arg)?)
                }
                "--progress-every" => {
                    progress_every = parse_usize(&next_value(&mut args, &arg)?, &arg)?
                }
                "--reset-db" => reset_db = true,
                "--help" | "-h" => return Err(usage()),
                value => return Err(format!("unknown option: {value}\n{}", usage())),
            }
        }
        Ok(Self {
            data_file: data_file.ok_or_else(|| "--data-file is required".to_owned())?,
            db_root: db_root.ok_or_else(|| "--db-root is required".to_owned())?,
            output: output.ok_or_else(|| "--output is required".to_owned())?,
            report,
            top_k,
            max_questions,
            progress_every,
            reset_db,
        })
    }
}

fn next_value(args: &mut impl Iterator<Item = String>, name: &str) -> Result<String, String> {
    args.next()
        .ok_or_else(|| format!("{name} requires a value"))
}

fn parse_usize(value: &str, name: &str) -> Result<usize, String> {
    value
        .parse::<usize>()
        .map_err(|_| format!("{name} must be an integer"))
}

fn usage() -> String {
    "usage: locomo_retrieval --data-file <json> --db-root <path> --output <jsonl> [--report <json>] [--top-k <n>] [--reset-db]".to_owned()
}
