use cortex_aql::AgentView;
use cortex_engine::Database;

use crate::authz;
use crate::responses::{AqlCellResponse, AqlResponse, RouterError};
use crate::router::query_param_decoded;

pub fn handle_aql_shared(
    db: &Database,
    query: &str,
    body: &[u8],
    authenticated_view: Option<&AgentView>,
) -> Result<String, RouterError> {
    let scope = query_param_decoded(query, "scope").map_err(RouterError::BadRequest)?;
    let aql = String::from_utf8_lossy(body);
    let view = authz::read_view_for_scope(&scope, authenticated_view)?;
    let cells = db.retrieve_aql(&aql, &view)?;
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
