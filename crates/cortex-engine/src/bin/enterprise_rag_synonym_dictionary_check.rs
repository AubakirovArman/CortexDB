use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use cortex_engine::search::{
    write_acsyn_dictionary, CorpusSynonymDictionary, CorpusSynonymDictionaryBuilder,
    CorpusSynonymOptions,
};
use serde_json::{json, Value};

#[derive(Clone, Debug, PartialEq, Eq)]
struct Args {
    uuid_index: PathBuf,
    sources_dir: PathBuf,
    output: PathBuf,
    report: PathBuf,
    limit: Option<usize>,
    min_terms_with_synonyms: usize,
    min_term_document_frequency: u32,
    min_pair_document_frequency: u32,
    max_synonyms_per_term: usize,
    max_terms: usize,
    max_terms_per_document: usize,
    progress_every: usize,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let args = parse_args(env::args().skip(1))?;
    let relative_paths = read_uuid_index_paths(&args.uuid_index, args.limit)?;
    let dictionary = build_dictionary_from_paths(
        &args.sources_dir,
        &relative_paths,
        synonym_options(&args),
        args.progress_every,
    );
    let dictionary = dictionary?;
    write_acsyn_dictionary(&args.output, &dictionary)
        .map_err(|error| format!("failed to write {}: {error}", args.output.display()))?;
    let report = report(&args, relative_paths.len(), &dictionary);
    if let Some(parent) = args.report.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    }
    let text = serde_json::to_string_pretty(&report)
        .map_err(|error| format!("failed to encode report: {error}"))?;
    fs::write(&args.report, format!("{text}\n"))
        .map_err(|error| format!("failed to write {}: {error}", args.report.display()))?;
    println!(
        "{}",
        json!({
            "documents": relative_paths.len(),
            "terms_with_synonyms": dictionary.terms_with_synonyms(),
            "entries": dictionary.entries.len(),
            "status": report.get("status").and_then(Value::as_str).unwrap_or("failed"),
            "output": args.output.display().to_string(),
            "report": args.report.display().to_string(),
        })
    );
    if report["status"] != "passed" {
        return Err(format!(
            "synonym dictionary check failed; see {}",
            args.report.display()
        ));
    }
    Ok(())
}

fn parse_args(mut args: impl Iterator<Item = String>) -> Result<Args, String> {
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

fn usage() -> String {
    "usage: enterprise_rag_synonym_dictionary_check --uuid-index <uuid_index.json> --sources-dir <generated_data/sources> --output <dictionary.acsyn> --report <report.json> [--limit N] [--min-terms-with-synonyms N] [--progress-every N]".to_owned()
}

fn synonym_options(args: &Args) -> CorpusSynonymOptions {
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

fn read_uuid_index_paths(path: &Path, limit: Option<usize>) -> Result<Vec<String>, String> {
    let text = fs::read_to_string(path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    let value: Value = serde_json::from_str(&text)
        .map_err(|error| format!("invalid JSON {}: {error}", path.display()))?;
    let object = value
        .as_object()
        .ok_or_else(|| "uuid index must be a JSON object".to_owned())?;
    let mut paths = object
        .values()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect::<Vec<_>>();
    paths.sort();
    paths.dedup();
    if let Some(limit) = limit {
        paths.truncate(limit);
    }
    Ok(paths)
}

fn build_dictionary_from_paths(
    sources_dir: &Path,
    relative_paths: &[String],
    options: CorpusSynonymOptions,
    progress_every: usize,
) -> Result<CorpusSynonymDictionary, String> {
    let mut builder = CorpusSynonymDictionaryBuilder::new();
    let total = relative_paths.len();
    for (index, relative_path) in relative_paths.iter().enumerate() {
        let document = read_document(sources_dir, relative_path)?;
        builder.add_document(&document, options);
        let done = index + 1;
        if progress_every > 0 && (done % progress_every == 0 || done == total) {
            eprintln!("processed {done}/{total} documents for corpus synonym dictionary");
        }
    }
    Ok(builder.finish(options))
}

fn read_document(sources_dir: &Path, relative_path: &str) -> Result<String, String> {
    let path = sources_dir.join(relative_path);
    let text = fs::read_to_string(&path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    let value: Value = serde_json::from_str(&text)
        .map_err(|error| format!("invalid JSON {}: {error}", path.display()))?;
    let mut parts = Vec::new();
    collect_json_strings(&value, &mut parts);
    Ok(parts.join("\n"))
}

fn collect_json_strings(value: &Value, out: &mut Vec<String>) {
    match value {
        Value::String(text) if useful_document_string(text) => out.push(text.clone()),
        Value::Array(values) => {
            for value in values {
                collect_json_strings(value, out);
            }
        }
        Value::Object(map) => {
            for (key, value) in map {
                if key == "dataset_doc_uuid" {
                    continue;
                }
                collect_json_strings(value, out);
            }
        }
        _ => {}
    }
}

fn useful_document_string(text: &str) -> bool {
    text.chars().filter(|ch| ch.is_ascii_alphabetic()).count() >= 3
}

fn report(args: &Args, document_count: usize, dictionary: &CorpusSynonymDictionary) -> Value {
    let terms_with_synonyms = dictionary.terms_with_synonyms();
    let mut errors = Vec::new();
    if terms_with_synonyms < args.min_terms_with_synonyms {
        errors.push(format!(
            "terms_with_synonyms {terms_with_synonyms} < {}",
            args.min_terms_with_synonyms
        ));
    }
    json!({
        "schema_version": "cortexdb.enterprise_rag_synonym_dictionary_check.v1",
        "documents": document_count,
        "entries": dictionary.entries.len(),
        "terms_with_synonyms": terms_with_synonyms,
        "min_terms_with_synonyms": args.min_terms_with_synonyms,
        "options": {
            "min_term_document_frequency": args.min_term_document_frequency,
            "min_pair_document_frequency": args.min_pair_document_frequency,
            "max_synonyms_per_term": args.max_synonyms_per_term,
            "max_terms": args.max_terms,
            "max_terms_per_document": args.max_terms_per_document,
            "progress_every": args.progress_every,
            "streaming_document_build": true,
        },
        "sample": dictionary.entries.iter().take(20).map(|entry| {
            json!({
                "term": entry.term,
                "document_frequency": entry.document_frequency,
                "synonyms": entry.synonyms.iter().map(|candidate| {
                    json!({
                        "term": candidate.term,
                        "score_q16": candidate.score_q16,
                        "cooccurrence_count": candidate.cooccurrence_count,
                    })
                }).collect::<Vec<_>>(),
            })
        }).collect::<Vec<_>>(),
        "errors": errors,
        "status": if errors.is_empty() { "passed" } else { "failed" },
    })
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::{build_dictionary_from_paths, parse_args, read_uuid_index_paths, synonym_options};

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

    #[test]
    fn uuid_index_paths_are_sorted_deduplicated_and_limited() {
        let path = std::env::temp_dir().join("cortexdb-uuid-index-test.json");
        std::fs::write(
            &path,
            r#"{"b":"z/doc.json","a":"a/doc.json","c":"a/doc.json"}"#,
        )
        .unwrap();

        let paths = read_uuid_index_paths(&path, Some(10)).unwrap();
        let _ = std::fs::remove_file(path);

        assert_eq!(paths, vec!["a/doc.json", "z/doc.json"]);
    }

    #[test]
    fn streams_documents_into_dictionary_without_batch_collection() {
        let root = unique_temp_dir("cortexdb-synonym-stream-test");
        let sources = root.join("sources");
        fs::create_dir_all(sources.join("a")).unwrap();
        fs::write(
            root.join("uuid_index.json"),
            r#"{"1":"a/doc1.json","2":"a/doc2.json","3":"a/doc3.json"}"#,
        )
        .unwrap();
        fs::write(
            sources.join("a/doc1.json"),
            r#"{"dataset_doc_uuid":"1","title":"Zephyr Quartz","body":"Zephyr quartz rollout"}"#,
        )
        .unwrap();
        fs::write(
            sources.join("a/doc2.json"),
            r#"{"dataset_doc_uuid":"2","body":"Zephyr quartz incident"}"#,
        )
        .unwrap();
        fs::write(
            sources.join("a/doc3.json"),
            r#"{"dataset_doc_uuid":"3","body":"Quartz migration note"}"#,
        )
        .unwrap();
        let args = parse_args(
            [
                "--uuid-index",
                root.join("uuid_index.json").to_str().unwrap(),
                "--sources-dir",
                sources.to_str().unwrap(),
                "--output",
                root.join("dictionary.acsyn").to_str().unwrap(),
                "--report",
                root.join("report.json").to_str().unwrap(),
                "--min-term-document-frequency",
                "2",
                "--min-pair-document-frequency",
                "2",
                "--progress-every",
                "0",
            ]
            .into_iter()
            .map(str::to_owned),
        )
        .unwrap();
        let paths = read_uuid_index_paths(&args.uuid_index, args.limit).unwrap();
        let dictionary =
            build_dictionary_from_paths(&args.sources_dir, &paths, synonym_options(&args), 0)
                .unwrap();
        let _ = fs::remove_dir_all(root);

        assert!(dictionary
            .synonyms_for("zephyr")
            .contains(&"quartz".to_owned()));
    }

    fn unique_temp_dir(prefix: &str) -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("{prefix}-{nanos}"))
    }
}
