use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use serde_json::{Map, Value};

pub fn read_json(path: &Path) -> Result<Value, String> {
    let text = fs::read_to_string(path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    serde_json::from_str(&text).map_err(|error| format!("invalid JSON {}: {error}", path.display()))
}

pub fn read_jsonl(path: &Path) -> Result<Vec<Value>, String> {
    let text = fs::read_to_string(path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    text.lines()
        .enumerate()
        .filter(|(_, line)| !line.trim().is_empty())
        .map(|(index, line)| {
            serde_json::from_str(line)
                .map_err(|error| format!("invalid JSONL {}:{}: {error}", path.display(), index + 1))
        })
        .collect()
}

pub fn read_uuid_index(path: &Path) -> Result<BTreeMap<String, String>, String> {
    let value = read_json(path)?;
    let object: Map<String, Value> = value
        .as_object()
        .cloned()
        .ok_or_else(|| format!("{}: expected JSON object", path.display()))?;
    object
        .into_iter()
        .map(|(key, value)| {
            value
                .as_str()
                .map(|path| (key, path.to_owned()))
                .ok_or_else(|| "uuid index values must be strings".to_owned())
        })
        .collect()
}

pub fn write_json(path: &Path, value: &Value) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    }
    fs::write(
        path,
        serde_json::to_string_pretty(value).map_err(|error| error.to_string())? + "\n",
    )
    .map_err(|error| format!("failed to write {}: {error}", path.display()))
}

pub fn write_jsonl(path: &Path, rows: &[Value]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    }
    let mut lines = String::new();
    for row in rows {
        lines.push_str(&serde_json::to_string(row).map_err(|error| error.to_string())?);
        lines.push('\n');
    }
    fs::write(path, lines).map_err(|error| format!("failed to write {}: {error}", path.display()))
}
