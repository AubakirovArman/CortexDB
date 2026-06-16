use std::fs;
use std::path::Path;

use cortex_engine::search::{
    CorpusSynonymDictionary, CorpusSynonymDictionaryBuilder, CorpusSynonymOptions,
};
use serde_json::Value;

pub(super) fn read_uuid_index_paths(
    path: &Path,
    limit: Option<usize>,
) -> Result<Vec<String>, String> {
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

pub(super) fn build_dictionary_from_paths(
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

#[cfg(test)]
mod tests {
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::{build_dictionary_from_paths, read_uuid_index_paths};
    use crate::args::{parse_args, synonym_options};

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
