#[cfg(test)]
pub(crate) fn build_benchmark_aql(
    query: &str,
    query_vector: Option<&Vec<i16>>,
    limit: usize,
) -> String {
    let task = benchmark_task(query, query_vector);
    let mode = if let Some(vector) = query_vector {
        let _ = vector;
        "semantic"
    } else {
        "balanced"
    };
    format!(
        "RETRIEVE CONTEXT FOR TASK {} IN BRAIN enterprise_rag USING MODE {mode} WHERE space = bench:enterprise_rag AND status = \"ready\" AND type = \"document_block\" LIMIT {} CANDIDATES;",
        quote_aql_string(&task),
        limit.max(1)
    )
}

pub(crate) fn benchmark_task(query: &str, query_vector: Option<&Vec<i16>>) -> String {
    let mut task = query.to_owned();
    if let Some(vector) = query_vector {
        task.push('\n');
        task.push_str("query_vector=");
        task.push_str(&vector_literal(vector));
    }
    task
}

fn vector_literal(vector: &[i16]) -> String {
    vector
        .iter()
        .map(i16::to_string)
        .collect::<Vec<_>>()
        .join(",")
}

#[cfg(test)]
pub(crate) fn quote_aql_string(value: &str) -> String {
    let mut quoted = String::with_capacity(value.len() + 2);
    quoted.push('"');
    for ch in value.chars() {
        match ch {
            '\\' => quoted.push_str("\\\\"),
            '"' => quoted.push_str("\\\""),
            '\n' => quoted.push_str("\\n"),
            '\r' => quoted.push_str("\\r"),
            '\t' => quoted.push_str("\\t"),
            ch => quoted.push(ch),
        }
    }
    quoted.push('"');
    quoted
}
