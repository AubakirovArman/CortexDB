use std::path::PathBuf;

#[cfg(test)]
mod tests;

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
    pub skip_checkpoint: bool,
    pub official_clean: bool,
    pub retrieval_mode: BenchmarkRetrievalMode,
    pub rerank_mode: BenchmarkRerankMode,
    pub query_vectors: Option<PathBuf>,
    pub document_vectors: Option<PathBuf>,
    pub prefilter_retrieval: Option<PathBuf>,
    pub disable_search_prefilter: bool,
    pub log_file: Option<PathBuf>,
    pub status_file: Option<PathBuf>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BenchmarkRetrievalMode {
    CachedLexical,
    EngineAql,
    EngineKeyword,
    EngineHybrid,
    EngineHybridRerank,
}

impl BenchmarkRetrievalMode {
    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "cached-lexical" => Ok(Self::CachedLexical),
            "engine-aql" => Ok(Self::EngineAql),
            "engine-keyword" => Ok(Self::EngineKeyword),
            "engine-hybrid" => Ok(Self::EngineHybrid),
            "engine-hybrid-rerank" => Ok(Self::EngineHybridRerank),
            _ => Err(
                "retrieval mode must be cached-lexical, engine-aql, engine-keyword, engine-hybrid, or engine-hybrid-rerank"
                    .to_owned(),
            ),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::CachedLexical => "cached-lexical",
            Self::EngineAql => "engine-aql",
            Self::EngineKeyword => "engine-keyword",
            Self::EngineHybrid => "engine-hybrid",
            Self::EngineHybridRerank => "engine-hybrid-rerank",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BenchmarkRerankMode {
    None,
    Weighted,
    /// A7.2: rerank the lexical candidate pool by the engine's own exact dense
    /// two-stage pass (`Database::two_stage_rerank_for_task`, pure dot).
    TwoStage,
    /// A7.2 cosine variant: `Database::two_stage_rerank_for_task_cosine` (dot
    /// normalized by candidate magnitude — ranks by direction, not raw dot).
    TwoStageCosine,
}

impl BenchmarkRerankMode {
    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "none" => Ok(Self::None),
            "weighted" => Ok(Self::Weighted),
            "two-stage" => Ok(Self::TwoStage),
            "two-stage-cosine" => Ok(Self::TwoStageCosine),
            _ => {
                Err("rerank mode must be none, weighted, two-stage, or two-stage-cosine".to_owned())
            }
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Weighted => "weighted",
            Self::TwoStage => "two-stage",
            Self::TwoStageCosine => "two-stage-cosine",
        }
    }

    pub fn is_enabled(self) -> bool {
        matches!(self, Self::Weighted | Self::TwoStage | Self::TwoStageCosine)
    }
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
                "--skip-checkpoint" => parsed.skip_checkpoint = true,
                "--official-clean" => parsed.official_clean = true,
                "--retrieval-mode" => {
                    parsed.retrieval_mode =
                        BenchmarkRetrievalMode::parse(&next_value(&mut args, &arg)?)?
                }
                "--rerank" => {
                    parsed.rerank_mode = BenchmarkRerankMode::parse(&next_value(&mut args, &arg)?)?
                }
                "--query-vectors" => {
                    parsed.query_vectors = Some(PathBuf::from(next_value(&mut args, &arg)?))
                }
                "--document-vectors" => {
                    parsed.document_vectors = Some(PathBuf::from(next_value(&mut args, &arg)?))
                }
                "--prefilter-retrieval" => {
                    parsed.prefilter_retrieval = Some(PathBuf::from(next_value(&mut args, &arg)?))
                }
                "--disable-search-prefilter" => parsed.disable_search_prefilter = true,
                "--log-file" => parsed.log_file = Some(PathBuf::from(next_value(&mut args, &arg)?)),
                "--status-file" => {
                    parsed.status_file = Some(PathBuf::from(next_value(&mut args, &arg)?))
                }
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
    skip_checkpoint: bool,
    official_clean: bool,
    retrieval_mode: BenchmarkRetrievalMode,
    rerank_mode: BenchmarkRerankMode,
    query_vectors: Option<PathBuf>,
    document_vectors: Option<PathBuf>,
    prefilter_retrieval: Option<PathBuf>,
    disable_search_prefilter: bool,
    log_file: Option<PathBuf>,
    status_file: Option<PathBuf>,
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
            skip_checkpoint: false,
            official_clean: false,
            retrieval_mode: BenchmarkRetrievalMode::CachedLexical,
            rerank_mode: BenchmarkRerankMode::None,
            query_vectors: None,
            document_vectors: None,
            prefilter_retrieval: None,
            disable_search_prefilter: false,
            log_file: None,
            status_file: None,
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
            skip_checkpoint: self.skip_checkpoint,
            official_clean: self.official_clean,
            retrieval_mode: self.retrieval_mode,
            rerank_mode: self.rerank_mode,
            query_vectors: self.query_vectors,
            document_vectors: self.document_vectors,
            prefilter_retrieval: self.prefilter_retrieval,
            disable_search_prefilter: self.disable_search_prefilter,
            log_file: self.log_file,
            status_file: self.status_file,
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
        "[--reset-db] [--skip-ingest] [--skip-checkpoint] [--official-clean] ",
        "[--retrieval-mode <cached-lexical|engine-aql|engine-keyword|engine-hybrid|engine-hybrid-rerank>] ",
        "[--rerank <none|weighted>] ",
        "[--query-vectors <jsonl>] [--document-vectors <jsonl>] ",
        "[--prefilter-retrieval <clean-jsonl>] [--disable-search-prefilter] ",
        "[--log-file <path>] [--status-file <path>]"
    )
    .to_owned()
}
