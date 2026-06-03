use std::path::PathBuf;

#[derive(Debug, PartialEq, Eq)]
pub struct Args {
    pub questions: PathBuf,
    pub uuid_index: PathBuf,
    pub sources_dir: PathBuf,
    pub db_root: PathBuf,
    pub output: PathBuf,
    pub report: Option<PathBuf>,
    pub top_k: usize,
    pub batch_size: usize,
    pub progress_every: usize,
    pub max_documents: Option<usize>,
    pub reset_db: bool,
    pub skip_ingest: bool,
}

impl Args {
    pub fn parse(args: impl IntoIterator<Item = String>) -> Result<Self, String> {
        let mut parsed = PartialArgs::default();
        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--questions" => {
                    parsed.questions = Some(PathBuf::from(next_value(&mut args, &arg)?))
                }
                "--uuid-index" => {
                    parsed.uuid_index = Some(PathBuf::from(next_value(&mut args, &arg)?))
                }
                "--sources-dir" => {
                    parsed.sources_dir = Some(PathBuf::from(next_value(&mut args, &arg)?))
                }
                "--db-root" => parsed.db_root = Some(PathBuf::from(next_value(&mut args, &arg)?)),
                "--output" => parsed.output = Some(PathBuf::from(next_value(&mut args, &arg)?)),
                "--report" => parsed.report = Some(PathBuf::from(next_value(&mut args, &arg)?)),
                "--top-k" => parsed.top_k = parse_usize(&next_value(&mut args, &arg)?, &arg)?,
                "--batch-size" => {
                    parsed.batch_size = parse_usize(&next_value(&mut args, &arg)?, &arg)?
                }
                "--progress-every" => {
                    parsed.progress_every = parse_usize(&next_value(&mut args, &arg)?, &arg)?
                }
                "--max-documents" => {
                    parsed.max_documents = Some(parse_usize(&next_value(&mut args, &arg)?, &arg)?)
                }
                "--reset-db" => parsed.reset_db = true,
                "--skip-ingest" => parsed.skip_ingest = true,
                "--help" | "-h" => return Err(usage()),
                value => return Err(format!("unknown option: {value}\n{}", usage())),
            }
        }
        parsed.finish()
    }
}

struct PartialArgs {
    questions: Option<PathBuf>,
    uuid_index: Option<PathBuf>,
    sources_dir: Option<PathBuf>,
    db_root: Option<PathBuf>,
    output: Option<PathBuf>,
    report: Option<PathBuf>,
    top_k: usize,
    batch_size: usize,
    progress_every: usize,
    max_documents: Option<usize>,
    reset_db: bool,
    skip_ingest: bool,
}

impl Default for PartialArgs {
    fn default() -> Self {
        Self {
            questions: None,
            uuid_index: None,
            sources_dir: None,
            db_root: None,
            output: None,
            report: None,
            top_k: 10,
            batch_size: 1_000,
            progress_every: 10_000,
            max_documents: None,
            reset_db: false,
            skip_ingest: false,
        }
    }
}

impl PartialArgs {
    fn finish(self) -> Result<Args, String> {
        Ok(Args {
            questions: self
                .questions
                .ok_or_else(|| "--questions is required".to_owned())?,
            uuid_index: self
                .uuid_index
                .ok_or_else(|| "--uuid-index is required".to_owned())?,
            sources_dir: self
                .sources_dir
                .ok_or_else(|| "--sources-dir is required".to_owned())?,
            db_root: self
                .db_root
                .ok_or_else(|| "--db-root is required".to_owned())?,
            output: self
                .output
                .ok_or_else(|| "--output is required".to_owned())?,
            report: self.report,
            top_k: self.top_k,
            batch_size: self.batch_size.max(1),
            progress_every: self.progress_every,
            max_documents: self.max_documents,
            reset_db: self.reset_db,
            skip_ingest: self.skip_ingest,
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
    concat!(
        "usage: enterprise_rag_bench_retrieval ",
        "--questions <jsonl> --uuid-index <json> --sources-dir <dir> ",
        "--db-root <path> --output <jsonl> [--report <json>] ",
        "[--top-k <n>] [--batch-size <n>] [--max-documents <n>] ",
        "[--reset-db] [--skip-ingest]"
    )
    .to_owned()
}
