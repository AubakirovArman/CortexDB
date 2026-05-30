use cortex_engine::Database;

use crate::context::view_for_scope;
use crate::responses::{AqlCellResponse, AqlResponse, RouterError};
use crate::router::query_param_decoded;

pub fn handle_aql_shared(db: &Database, query: &str, body: &[u8]) -> Result<String, RouterError> {
    let scope = query_param_decoded(query, "scope").map_err(RouterError::BadRequest)?;
    let aql = String::from_utf8_lossy(body);
    let cells = db.retrieve_aql(&aql, &view_for_scope(&scope))?;
    let response = AqlResponse {
        cells: cells
            .iter()
            .map(|cell| AqlCellResponse {
                cell_id: cell.cell_id.0,
                payload: String::from_utf8_lossy(&cell.payload).into_owned(),
            })
            .collect(),
    };
    Ok(serde_json::to_string(&response)?)
}
