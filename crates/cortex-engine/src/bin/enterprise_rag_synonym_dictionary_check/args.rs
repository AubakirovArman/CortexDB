use std::path::PathBuf;

use cortex_engine::search::CorpusSynonymOptions;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Args {
    pub(super) uuid_index: PathBuf,
    pub(super) sources_dir: PathBuf,
    pub(super) output: PathBuf,
    pub(super) report: PathBuf,
    pub(super) limit: Option<usize>,
    pub(super) min_terms_with_synonyms: usize,
    pub(super) min_term_document_frequency: u32,
    pub(super) min_pair_document_frequency: u32,
    pub(super) max_synonyms_per_term: usize,
    pub(super) max_terms: usize,
    pub(super) max_terms_per_document: usize,
    pub(super) progress_every: usize,
}

pub(super) fn parse_args(mut args: impl Iterator<Item = String>) -> Result<Args, String> {
    let mut uuid_index = None;
    let mut sources_dir = None;
    let mut output = None;
    let mut report = None;
    let mut limit = None;
    let mut min_terms_with_synonyms = 1_000usize;
    let mut min_term_document_frequency = 3u32;
    let mut min_pair_document_frequency = 2u32;
    let mut max_synonyms_per_term = 8usize;
    let mut max_terms = 10_000usize;
    let mut max_terms_per_document = 64usize;
    let mut progress_every = 10_000usize;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--uuid-index" => uuid_index = Some(PathBuf::from(next_value(&mut args, &arg)?)),
            "--sources-dir" => sources_dir = Some(PathBuf::from(next_value(&mut args, &arg)?)),
            "--output" => output = Some(PathBuf::from(next_value(&mut args, &arg)?)),
            "--report" => report = Some(PathBuf::from(next_value(&mut args, &arg)?)),
            "--limit" => limit = Some(parse_usize(&next_value(&mut args, &arg)?, &arg)?),
            "--min-terms-with-synonyms" => {
                min_terms_with_synonyms = parse_usize(&next_value(&mut args, &arg)?, &arg)?
            }
            "--min-term-document-frequency" => {
                min_term_document_frequency = parse_u32(&next_value(&mut args, &arg)?, &arg)?
            }
            "--min-pair-document-frequency" => {
                min_pair_document_frequency = parse_u32(&next_value(&mut args, &arg)?, &arg)?
            }
            "--max-synonyms-per-term" => {
                max_synonyms_per_term = parse_usize(&next_value(&mut args, &arg)?, &arg)?
            }
            "--max-terms" => max_terms = parse_usize(&next_value(&mut args, &arg)?, &arg)?,
            "--max-terms-per-document" => {
                max_terms_per_document = parse_usize(&next_value(&mut args, &arg)?, &arg)?
            }
            "--progress-every" => {
                progress_every = parse_usize(&next_value(&mut args, &arg)?, &arg)?
            }
            "--help" | "-h" => return Err(usage()),
            _ => return Err(format!("unknown argument {arg}\n{}", usage())),
        }
    }
    Ok(Args {
        uuid_index: uuid_index.ok_or_else(usage)?,
        sources_dir: sources_dir.ok_or_else(usage)?,
        output: output.ok_or_else(usage)?,
        report: report.ok_or_else(usage)?,
        limit,
        min_terms_with_synonyms,
        min_term_document_frequency,
        min_pair_document_frequency,
        max_synonyms_per_term,
        max_terms,
        max_terms_per_document,
        progress_every,
    })
}

pub(super) fn synonym_options(args: &Args) -> CorpusSynonymOptions {
    CorpusSynonymOptions {
        min_term_document_frequency: args.min_term_document_frequency,
        min_pair_document_frequency: args.min_pair_document_frequency,
        max_synonyms_per_term: args.max_synonyms_per_term,
        max_terms: args.max_terms,
        max_terms_per_document: args.max_terms_per_document,
    }
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

fn parse_u32(value: &str, name: &str) -> Result<u32, String> {
    value
        .parse::<u32>()
        .map_err(|error| format!("{name} expects a positive integer: {error}"))
}

fn usage() -> String {
    "usage: enterprise_rag_synonym_dictionary_check --uuid-index <uuid_index.json> --sources-dir <generated_data/sources> --output <dictionary.acsyn> --report <report.json> [--limit N] [--min-terms-with-synonyms N] [--progress-every N]".to_owned()
}

#[cfg(test)]
mod tests {
    use super::parse_args;

    #[test]
    fn parse_args_accepts_required_paths_and_limit() {
        let args = parse_args(
            [
                "--uuid-index",
                "uuid_index.json",
                "--sources-dir",
                "sources",
                "--output",
                "dictionary.acsyn",
                "--report",
                "report.json",
                "--limit",
                "100",
            ]
            .into_iter()
            .map(str::to_owned),
        )
        .unwrap();

        assert_eq!(args.limit, Some(100));
        assert_eq!(args.min_terms_with_synonyms, 1_000);
        assert_eq!(args.progress_every, 10_000);
    }
}
