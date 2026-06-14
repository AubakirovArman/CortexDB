use crate::error::{EngineError, EngineResult};
use crate::ingestion::chunking::JsonChunkPolicy;

pub(crate) fn flat_json_fields(json: &str) -> EngineResult<Vec<(String, String)>> {
    flat_json_fields_with_policy(json, JsonChunkPolicy::default())
}

pub fn count_json_ingest_cells(json: &str) -> EngineResult<usize> {
    flat_json_fields(json).map(|fields| fields.len())
}

pub fn count_csv_ingest_cells(csv: &str) -> EngineResult<usize> {
    csv_rows(csv).map(|rows| rows.len().saturating_sub(1))
}

pub(crate) fn flat_json_fields_with_policy(
    json: &str,
    policy: JsonChunkPolicy,
) -> EngineResult<Vec<(String, String)>> {
    let policy = policy.validate()?;
    let parsed: serde_json::Value = serde_json::from_str(json)
        .map_err(|e| EngineError::StorageInvariant(format!("invalid json: {e}")))?;
    let mut out = Vec::new();
    flatten_json_value(&parsed, "", policy, &mut out);
    if policy.sort_paths {
        out.sort_by(|left, right| left.0.cmp(&right.0));
    }
    Ok(out)
}

pub(crate) fn csv_rows(csv: &str) -> EngineResult<Vec<Vec<String>>> {
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(false)
        .from_reader(csv.as_bytes());
    let mut rows = Vec::new();
    for result in rdr.records() {
        let record =
            result.map_err(|e| EngineError::StorageInvariant(format!("csv error: {e}")))?;
        rows.push(record.iter().map(|field| field.trim().to_owned()).collect());
    }
    Ok(rows)
}

fn flatten_json_value(
    value: &serde_json::Value,
    prefix: &str,
    policy: JsonChunkPolicy,
    out: &mut Vec<(String, String)>,
) {
    match value {
        serde_json::Value::Object(map) => {
            for (key, child) in map {
                let next_prefix = policy.join_path(prefix, key);
                flatten_json_value(child, &next_prefix, policy, out);
            }
        }
        serde_json::Value::Array(values) => {
            for (index, child) in values.iter().enumerate() {
                let next_prefix = policy.join_path(prefix, &index.to_string());
                flatten_json_value(child, &next_prefix, policy, out);
            }
        }
        serde_json::Value::String(value) => {
            out.push((prefix.to_owned(), value.clone()));
        }
        value => {
            out.push((prefix.to_owned(), value.to_string()));
        }
    }
}
