use std::fs;
use std::path::PathBuf;

use serde::Deserialize;

pub(super) fn read_jsonl<T: for<'de> Deserialize<'de>>(path: &PathBuf) -> Result<Vec<T>, String> {
    let text = fs::read_to_string(path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    let mut values = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        values.push(serde_json::from_str(line).map_err(|error| {
            format!(
                "{} line {} is invalid JSON: {error}",
                path.display(),
                index + 1
            )
        })?);
    }
    Ok(values)
}
