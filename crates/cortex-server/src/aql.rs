use cortex_engine::{Database, RetrievedCell};

use crate::context::view_for_scope;

pub fn handle_aql_shared(
    db: &std::sync::RwLock<Database>,
    query: &str,
    body: &[u8],
) -> Result<String, String> {
    let scope = query_param(query, "scope")?;
    let db = db.read().map_err(|e| e.to_string())?;
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
    let response = serde_json::json!({
        "cells": cells.iter().map(|cell| {
            serde_json::json!({
                "cell_id": cell.cell_id.0,
                "payload": String::from_utf8_lossy(&cell.payload).into_owned()
            })
        }).collect::<Vec<_>>()
    });
    serde_json::to_string(&response).unwrap_or_default()
}
