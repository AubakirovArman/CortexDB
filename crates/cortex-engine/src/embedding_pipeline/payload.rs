use super::types::embedding_ref_string;
use crate::ingestion::stable_ingestion_hash_hex;

pub fn embedding_text_hash(payload: &[u8]) -> String {
    stable_ingestion_hash_hex(embedding_text_for_hash(payload).as_bytes())
}

pub(super) fn embedding_text_for_hash(payload: &[u8]) -> String {
    let text = String::from_utf8_lossy(payload);
    text.lines()
        .filter(|line| {
            let trimmed = line.trim();
            !is_embedding_payload_line(trimmed)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub(super) fn embedding_payload_field(payload: &[u8], field: &str) -> Option<String> {
    let prefix = format!("{field}=");
    String::from_utf8_lossy(payload)
        .lines()
        .find_map(|line| line.trim().strip_prefix(&prefix).map(ToOwned::to_owned))
        .filter(|value| !value.is_empty())
}

pub(crate) fn payload_with_embedding(
    payload: &[u8],
    model: Option<&str>,
    text_hash: &str,
    vector: &[i16],
    metric: u32,
) -> Vec<u8> {
    let text = String::from_utf8_lossy(payload);
    let mut lines = text
        .lines()
        .filter(|line| !is_embedding_payload_line(line.trim()))
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    let mut embedding_lines = Vec::new();
    if let Some(model) = model {
        embedding_lines.push(format!(
            "embedding_model={}",
            sanitize_embedding_value(model)
        ));
    }
    embedding_lines.push(format!(
        "embedding_text_hash={}",
        sanitize_embedding_value(text_hash)
    ));
    embedding_lines.push(format!(
        "embedding_ref={}",
        sanitize_embedding_value(&embedding_ref_string(
            model.unwrap_or(""),
            vector.len() as u32,
            metric,
            text_hash,
        ))
    ));
    embedding_lines.push(format!("vector={}", format_vector_literal(vector)));
    let insert_at = lines
        .iter()
        .position(|line| line.trim().is_empty())
        .unwrap_or(0);
    for (offset, line) in embedding_lines.into_iter().enumerate() {
        lines.insert(insert_at + offset, line);
    }
    (lines.join("\n") + "\n").into_bytes()
}

fn is_embedding_payload_line(line: &str) -> bool {
    line.starts_with("vector=")
        || line.starts_with("embedding_model=")
        || line.starts_with("embedding_text_hash=")
        || line.starts_with("embedding_ref=")
        || line.contains("_vector=")
}

fn format_vector_literal(vector: &[i16]) -> String {
    vector
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(",")
}

fn sanitize_embedding_value(value: &str) -> String {
    value
        .chars()
        .map(|character| match character {
            '\n' | '\r' => ' ',
            other => other,
        })
        .collect()
}
