use std::fs;
use std::path::Path;

use serde_json::Value;

#[derive(Clone, Debug)]
pub(super) struct QuestionRow {
    pub(super) question_id: String,
    pub(super) question: String,
    pub(super) question_type: String,
    pub(super) answer_fact_count: usize,
    pub(super) expected_doc_count: usize,
}

pub(super) fn read_questions(path: &Path) -> Result<Vec<QuestionRow>, String> {
    let text = fs::read_to_string(path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    let mut rows = Vec::new();
    for (line_index, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let value: Value = serde_json::from_str(line)
            .map_err(|error| format!("invalid JSONL line {}: {error}", line_index + 1))?;
        let question_id = required_str(&value, "question_id", line_index)?;
        let question = required_str(&value, "question", line_index)?;
        let question_type = value
            .get("question_type")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .to_owned();
        let answer_fact_count = value
            .get("answer_facts")
            .and_then(Value::as_array)
            .map_or(0, Vec::len);
        let expected_doc_count = value
            .get("expected_doc_ids")
            .and_then(Value::as_array)
            .map_or(0, Vec::len);
        rows.push(QuestionRow {
            question_id,
            question,
            question_type,
            answer_fact_count,
            expected_doc_count,
        });
    }
    if rows.is_empty() {
        return Err("questions file is empty".to_owned());
    }
    Ok(rows)
}

fn required_str(value: &Value, key: &str, line_index: usize) -> Result<String, String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .filter(|text| !text.trim().is_empty())
        .map(str::to_owned)
        .ok_or_else(|| format!("line {} missing string field {key}", line_index + 1))
}
