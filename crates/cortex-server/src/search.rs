use std::path::Path;

use cortex_engine::{Database, SearchLimit};

use crate::context::{escape_json, view_for_scope};

pub fn handle_search(root: &Path, query: &str, body: &[u8]) -> Result<String, String> {
    let scope = query_param(query, "scope")?;
    let text = query_param(query, "q")
        .map(str::to_owned)
        .unwrap_or_else(|_| String::from_utf8_lossy(body).into_owned());
    let limit = query_param(query, "limit")
        .ok()
        .map(parse_limit)
        .transpose()?
        .unwrap_or(20);
    let db = Database::open(root).map_err(|error| error.to_string())?;
    let results = db
        .search_keyword(&text, &view_for_scope(scope), SearchLimit(limit))
        .map_err(|error| error.to_string())?;
    Ok(format!(
        r#"{{"results":[{}]}}"#,
        results
            .iter()
            .map(|result| {
                format!(
                    r#"{{"cell_id":{},"score":{},"lexical_score":{},"vector_score":{},"payload":"{}"}}"#,
                    result.cell_id.0,
                    result.score,
                    result.lexical_score,
                    result.vector_score,
                    escape_json(&String::from_utf8_lossy(&result.payload))
                )
            })
            .collect::<Vec<_>>()
            .join(",")
    ))
}

fn parse_limit(value: &str) -> Result<usize, String> {
    value
        .parse::<usize>()
        .map(|limit| limit.max(1))
        .map_err(|_| "limit must be usize".to_owned())
}

fn query_param<'a>(query: &'a str, key: &str) -> Result<&'a str, String> {
    let prefix = format!("{key}=");
    query
        .split('&')
        .find_map(|pair| pair.strip_prefix(&prefix))
        .ok_or_else(|| format!("missing {key}"))
}
