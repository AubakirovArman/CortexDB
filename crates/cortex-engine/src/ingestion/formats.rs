use crate::error::{EngineError, EngineResult};

pub(crate) fn flat_json_fields(json: &str) -> EngineResult<Vec<(String, String)>> {
    let parsed: serde_json::Value = serde_json::from_str(json)
        .map_err(|e| EngineError::StorageInvariant(format!("invalid json: {e}")))?;
    let mut out = Vec::new();
    flatten_json_value(&parsed, "", &mut out);
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

fn flatten_json_value(value: &serde_json::Value, prefix: &str, out: &mut Vec<(String, String)>) {
    match value {
        serde_json::Value::Object(map) => {
            for (key, child) in map {
                let next_prefix = if prefix.is_empty() {
                    key.clone()
                } else {
                    format!("{prefix}.{key}")
                };
                flatten_json_value(child, &next_prefix, out);
            }
        }
        serde_json::Value::Array(values) => {
            for (index, child) in values.iter().enumerate() {
                flatten_json_value(child, &format!("{prefix}.{index}"), out);
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
