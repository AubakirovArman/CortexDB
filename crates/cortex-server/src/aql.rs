use std::path::Path;

use cortex_engine::{Database, RetrievedCell};

use crate::context::{escape_json, view_for_scope};

pub fn handle_aql(root: &Path, query: &str, body: &[u8]) -> Result<String, String> {
    let scope = query_param(query, "scope")?;
    let db = Database::open(root).map_err(|error| error.to_string())?;
    let aql = String::from_utf8_lossy(body);
    let cells = db
        .retrieve_aql(&aql, &view_for_scope(scope))
        .map_err(|error| error.to_string())?;
    Ok(cells_json(&cells))
}

fn query_param<'a>(query: &'a str, key: &str) -> Result<&'a str, String> {
    let prefix = format!("{key}=");
    query
        .split('&')
        .find_map(|pair| pair.strip_prefix(&prefix))
        .ok_or_else(|| format!("missing {key}"))
}

fn cells_json(cells: &[RetrievedCell]) -> String {
    let cells = cells
        .iter()
        .map(|cell| {
            format!(
                r#"{{"cell_id":{},"payload":"{}"}}"#,
                cell.cell_id.0,
                escape_json(&String::from_utf8_lossy(&cell.payload))
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!(r#"{{"cells":[{cells}]}}"#)
}
