use crate::responses::RouterError;

use super::client::format_vector_literal;

pub(crate) fn semantic_aql_needs_query_vector(aql: &str) -> bool {
    aql.to_ascii_lowercase().contains("using mode semantic") && !aql_task_has_query_vector(aql)
}

pub(crate) fn aql_task_has_query_vector(aql: &str) -> bool {
    retrieve_task_text(aql)
        .as_deref()
        .map(task_has_vector_line)
        .unwrap_or(false)
}

pub(crate) fn retrieve_task_text(aql: &str) -> Option<String> {
    let (start, end) = task_content_bounds(aql)?;
    unescape_aql_string(&aql[start..end])
}

pub(crate) fn inject_query_vector(aql: &str, vector: &[i16]) -> Result<String, RouterError> {
    if vector.is_empty() {
        return Err(RouterError::BadRequest(
            "embedding provider returned empty vector".to_owned(),
        ));
    }
    let (start, _) = task_content_bounds(aql).ok_or_else(|| {
        RouterError::BadRequest("embed_query requires RETRIEVE CONTEXT FOR TASK AQL".to_owned())
    })?;
    let mut rewritten = String::with_capacity(aql.len() + vector.len().saturating_mul(7) + 16);
    rewritten.push_str(&aql[..start]);
    rewritten.push_str("query_vector=");
    rewritten.push_str(&format_vector_literal(vector));
    rewritten.push_str("\\n");
    rewritten.push_str(&aql[start..]);
    Ok(rewritten)
}

fn task_has_vector_line(task: &str) -> bool {
    task.lines().any(|line| {
        let trimmed = line.trim();
        trimmed.starts_with("query_vector=") || trimmed.starts_with("vector=")
    })
}

fn task_content_bounds(aql: &str) -> Option<(usize, usize)> {
    let lower = aql.to_ascii_lowercase();
    let marker = lower.find("for task")?;
    let mut offset = marker + "for task".len();
    let bytes = aql.as_bytes();
    while matches!(bytes.get(offset), Some(b' ' | b'\n' | b'\r' | b'\t')) {
        offset += 1;
    }
    if bytes.get(offset) != Some(&b'"') {
        return None;
    }
    let start = offset + 1;
    let mut index = start;
    let mut escaped = false;
    while let Some(byte) = bytes.get(index).copied() {
        if escaped {
            escaped = false;
        } else if byte == b'\\' {
            escaped = true;
        } else if byte == b'"' {
            return Some((start, index));
        }
        index += 1;
    }
    None
}

fn unescape_aql_string(raw: &str) -> Option<String> {
    let mut out = String::with_capacity(raw.len());
    let mut chars = raw.chars();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            out.push(ch);
            continue;
        }
        match chars.next()? {
            '"' => out.push('"'),
            '\\' => out.push('\\'),
            'n' => out.push('\n'),
            'r' => out.push('\r'),
            't' => out.push('\t'),
            _ => return None,
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inject_query_vector_adds_literal_to_task() {
        let aql = r#"RETRIEVE CONTEXT FOR TASK "find budget" IN BRAIN default USING MODE semantic LIMIT 2 CANDIDATES;"#;

        let rewritten = inject_query_vector(aql, &[0, 100]).unwrap();

        assert!(rewritten.contains(r#"FOR TASK "query_vector=0,100\nfind budget""#));
        assert_eq!(
            retrieve_task_text(&rewritten).unwrap(),
            "query_vector=0,100\nfind budget"
        );
    }

    #[test]
    fn semantic_aql_without_query_vector_needs_embedding() {
        let without_vector = r#"RETRIEVE CONTEXT FOR TASK "find budget" IN BRAIN default USING MODE semantic LIMIT 2 CANDIDATES;"#;
        let with_vector = r#"RETRIEVE CONTEXT FOR TASK "query_vector=1,2\nfind budget" IN BRAIN default USING MODE semantic LIMIT 2 CANDIDATES;"#;

        assert!(semantic_aql_needs_query_vector(without_vector));
        assert!(!semantic_aql_needs_query_vector(with_vector));
    }
}
