use cortex_engine::Database;

use crate::context::view_for_scope;
use crate::responses::{AqlCellResponse, AqlResponse};
use crate::router::query_param_decoded;

pub fn handle_aql_shared(db: &Database, query: &str, body: &[u8]) -> Result<String, String> {
    let scope = query_param_decoded(query, "scope")?;
    let aql = String::from_utf8_lossy(body);
    let cells = db
        .retrieve_aql(&aql, &view_for_scope(&scope))
        .map_err(|error| error.to_string())?;
    let response = AqlResponse {
        cells: cells
            .iter()
            .map(|cell| AqlCellResponse {
                cell_id: cell.cell_id.0,
                payload: String::from_utf8_lossy(&cell.payload).into_owned(),
            })
            .collect(),
    };
    serde_json::to_string(&response).map_err(|e| e.to_string())
}
