use serde_json::Value;

pub fn extract_document_content(document: &Value) -> (String, String) {
    let Some(object) = document.as_object() else {
        return (String::new(), document.to_string());
    };
    let title = object
        .get("title_field_name")
        .and_then(Value::as_str)
        .and_then(|field| object.get(field))
        .map(value_to_text)
        .unwrap_or_default();
    let Some(fields) = object.get("content_field_names").and_then(Value::as_array) else {
        return (title, document.to_string());
    };

    let mut parts = Vec::new();
    for field in fields.iter().filter_map(Value::as_str) {
        if let Some(value) = object.get(field) {
            let text = value_to_text(value);
            if fields.len() == 1 {
                parts.push(text);
            } else {
                parts.push(format!("{field}:\n{text}"));
            }
        }
    }
    (title, parts.join("\n\n"))
}

pub fn build_payload(doc_id: &str, rel_path: &str, title: &str, content: &str) -> String {
    [
        "scope=bench:enterprise_rag".to_owned(),
        "status=ready".to_owned(),
        "type=document_block".to_owned(),
        format!("doc_id={doc_id}"),
        format!(
            "source_type={}",
            rel_path.split('/').next().unwrap_or("unknown")
        ),
        format!("rel_path={}", clean_line(rel_path)),
        format!("title={}", clean_line(title)),
        String::new(),
        title.to_owned(),
        content.to_owned(),
    ]
    .join("\n")
}

pub fn payload_field(payload: &str, field: &str) -> Option<String> {
    let prefix = format!("{field}=");
    payload
        .lines()
        .find_map(|line| line.strip_prefix(&prefix).map(str::to_owned))
}

fn clean_line(value: &str) -> String {
    value.replace(['\r', '\n'], " ").trim().to_owned()
}

fn value_to_text(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        Value::Array(items) => items
            .iter()
            .map(value_to_text)
            .collect::<Vec<_>>()
            .join("\n"),
        Value::Object(_) => value.to_string(),
        Value::Null => String::new(),
        _ => value.to_string(),
    }
}
