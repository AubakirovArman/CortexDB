use super::Args;

fn required_args() -> Vec<String> {
    [
        "--questions",
        "questions.jsonl",
        "--uuid-index",
        "uuid_index.json",
        "--sources-dir",
        "sources",
        "--db-root",
        "db",
        "--output",
        "retrieval.jsonl",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect()
}

#[test]
fn parses_official_clean_flag() {
    let mut args = required_args();
    args.push("--official-clean".to_owned());

    let parsed = Args::parse(args).expect("parse args");

    assert!(parsed.official_clean);
}

#[test]
fn official_clean_is_off_by_default() {
    let parsed = Args::parse(required_args()).expect("parse args");

    assert!(!parsed.official_clean);
}

#[test]
fn parses_skip_checkpoint_flag() {
    let mut args = required_args();
    args.push("--skip-checkpoint".to_owned());

    let parsed = Args::parse(args).expect("parse args");

    assert!(parsed.skip_checkpoint);
}

#[test]
fn parses_disable_search_prefilter_flag() {
    let mut args = required_args();
    args.push("--disable-search-prefilter".to_owned());

    let parsed = Args::parse(args).expect("parse args");

    assert!(parsed.disable_search_prefilter);
}

#[test]
fn parses_engine_hybrid_mode_and_query_vectors() {
    let mut args = required_args();
    args.extend([
        "--retrieval-mode".to_owned(),
        "engine-hybrid".to_owned(),
        "--query-vectors".to_owned(),
        "vectors.jsonl".to_owned(),
        "--document-vectors".to_owned(),
        "document_vectors.jsonl".to_owned(),
        "--prefilter-retrieval".to_owned(),
        "prefilter.jsonl".to_owned(),
    ]);

    let parsed = Args::parse(args).expect("parse args");

    assert_eq!(
        parsed.retrieval_mode,
        super::BenchmarkRetrievalMode::EngineHybrid
    );
    assert_eq!(
        parsed.query_vectors.unwrap(),
        std::path::PathBuf::from("vectors.jsonl")
    );
    assert_eq!(
        parsed.document_vectors.unwrap(),
        std::path::PathBuf::from("document_vectors.jsonl")
    );
    assert_eq!(
        parsed.prefilter_retrieval.unwrap(),
        std::path::PathBuf::from("prefilter.jsonl")
    );
}

#[test]
fn parses_engine_hybrid_rerank_mode() {
    let mut args = required_args();
    args.extend([
        "--retrieval-mode".to_owned(),
        "engine-hybrid-rerank".to_owned(),
        "--query-vectors".to_owned(),
        "vectors.jsonl".to_owned(),
    ]);

    let parsed = Args::parse(args).expect("parse args");

    assert_eq!(
        parsed.retrieval_mode,
        super::BenchmarkRetrievalMode::EngineHybridRerank
    );
    assert_eq!(parsed.retrieval_mode.as_str(), "engine-hybrid-rerank");
}

#[test]
fn parses_engine_aql_mode() {
    let mut args = required_args();
    args.extend(["--retrieval-mode".to_owned(), "engine-aql".to_owned()]);

    let parsed = Args::parse(args).expect("parse args");

    assert_eq!(
        parsed.retrieval_mode,
        super::BenchmarkRetrievalMode::EngineAql
    );
    assert_eq!(parsed.retrieval_mode.as_str(), "engine-aql");
}

#[test]
fn rerank_is_disabled_by_default() {
    let parsed = Args::parse(required_args()).expect("parse args");

    assert_eq!(parsed.rerank_mode, super::BenchmarkRerankMode::None);
    assert!(!parsed.rerank_mode.is_enabled());
}

#[test]
fn parses_weighted_rerank_mode() {
    let mut args = required_args();
    args.extend(["--rerank".to_owned(), "weighted".to_owned()]);

    let parsed = Args::parse(args).expect("parse args");

    assert_eq!(parsed.rerank_mode, super::BenchmarkRerankMode::Weighted);
    assert!(parsed.rerank_mode.is_enabled());
    assert_eq!(parsed.rerank_mode.as_str(), "weighted");
}

#[test]
fn rejects_unknown_rerank_mode() {
    let mut args = required_args();
    args.extend(["--rerank".to_owned(), "bogus".to_owned()]);

    let error = Args::parse(args).expect_err("unknown rerank mode should fail");

    assert!(error.contains("rerank mode must be"));
}

#[test]
fn parses_progress_log_paths() {
    let mut args = required_args();
    args.extend([
        "--log-file".to_owned(),
        "run.log".to_owned(),
        "--status-file".to_owned(),
        "status.json".to_owned(),
    ]);

    let parsed = Args::parse(args).expect("parse args");

    assert_eq!(
        parsed.log_file.unwrap(),
        std::path::PathBuf::from("run.log")
    );
    assert_eq!(
        parsed.status_file.unwrap(),
        std::path::PathBuf::from("status.json")
    );
}
