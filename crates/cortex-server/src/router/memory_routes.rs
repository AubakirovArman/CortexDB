use cortex_aql::AgentView;

use crate::authz;
use crate::memory;
use crate::responses::{PutCellResponse, RouterError};

use super::params::cell_id;
use super::DatabaseAccess;

pub(super) fn try_route<A: DatabaseAccess>(
    db: &mut A,
    method: &str,
    path: &str,
    query: &str,
    body: &[u8],
    authenticated_view: Option<&AgentView>,
) -> Option<Result<String, RouterError>> {
    if !matches!(
        (method, path),
        ("POST", "/v1/remember") | ("POST", "/v1/forget") | ("POST", "/v1/verify")
    ) {
        return None;
    }
    Some((|| -> Result<String, RouterError> {
        match (method, path) {
            ("POST", "/v1/remember") => {
                let db = db
                    .as_write()
                    .ok_or_else(|| RouterError::Internal("write route on read lock".to_owned()))?;
                memory::handle_remember_shared(db, query, body, authenticated_view)
            }
            ("POST", "/v1/forget") => {
                let db = db
                    .as_write()
                    .ok_or_else(|| RouterError::Internal("write route on read lock".to_owned()))?;
                let cell_id = cell_id(query)?;
                if let Some((_, descriptor)) = db.get_latest_cell_with_descriptor(cell_id) {
                    authz::require_descriptor_write(authenticated_view, &descriptor)?;
                }
                db.forget_cell(cell_id)?;
                Ok(serde_json::to_string(&PutCellResponse {
                    seq: 0,
                    cell_id: cell_id.0,
                })?)
            }
            ("POST", "/v1/verify") => {
                let db = db.as_read();
                memory::handle_verify_shared(db, query, body, authenticated_view)
            }
            _ => unreachable!("route prefiltered"),
        }
    })())
}
