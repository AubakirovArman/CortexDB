use std::env;
use std::fs;

use cortex_engine::search::write_acsyn_dictionary;
use serde_json::{json, Value};

#[path = "enterprise_rag_synonym_dictionary_check/args.rs"]
mod args;
#[path = "enterprise_rag_synonym_dictionary_check/corpus.rs"]
mod corpus;
#[path = "enterprise_rag_synonym_dictionary_check/report.rs"]
mod report;

use args::{parse_args, synonym_options};
use corpus::{build_dictionary_from_paths, read_uuid_index_paths};
use report::build_synonym_report;

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
    let report = build_synonym_report(&args, relative_paths.len(), &dictionary);
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
