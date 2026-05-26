use cortex_engine::{parse_vector_literal, Database, SearchLimit};

use crate::context::view_for_scope;
use crate::responses::{SearchResponse, SearchResultResponse};

pub fn handle_search_shared(
    db: &std::sync::RwLock<Database>,
    query: &str,
    body: &[u8],
) -> Result<String, String> {
    let scope = query_param(query, "scope")?;
    let limit = query_param(query, "limit")
        .ok()
        .map(parse_limit)
        .transpose()?
        .unwrap_or(20);
    let mode = query_param(query, "mode").unwrap_or("keyword");
    let algorithm = query_param(query, "algorithm").unwrap_or("ann");
    let db = db.read().map_err(|e| e.to_string())?;
    let (search_mode, results) = match mode {
        "keyword" => ("keyword", {
            let text = query_param(query, "q")
                .map(str::to_owned)
                .unwrap_or_else(|_| String::from_utf8_lossy(body).into_owned());
            db.search_keyword(&text, &view_for_scope(scope), SearchLimit(limit))
        }),
        "vector" => {
            let vector = query_param(query, "vector")
                .map(str::to_owned)
                .unwrap_or_else(|_| String::from_utf8_lossy(body).into_owned());
            let vector = parse_vector_literal(&vector)?;
            match algorithm {
                "exact" => (
                    "vector_exact",
                    db.search_vector_exact(&vector, &view_for_scope(scope), SearchLimit(limit)),
                ),
                "ann" => (
                    "vector_ann",
                    db.search_vector(&vector, &view_for_scope(scope), SearchLimit(limit)),
                ),
                _ => return Err("algorithm must be exact or ann".to_owned()),
            }
        }
        _ => return Err("mode must be keyword or vector".to_owned()),
    };
    let results = results.map_err(|error| error.to_string())?;

    let response = SearchResponse {
        search_mode: search_mode.to_owned(),
        results: results
            .iter()
            .map(|result| SearchResultResponse {
                cell_id: result.cell_id.0,
                score: result.score,
                lexical_score: result.lexical_score,
                vector_score: result.vector_score,
                payload: String::from_utf8_lossy(&result.payload).into_owned(),
            })
            .collect(),
    };
    serde_json::to_string(&response).map_err(|e| e.to_string())
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
