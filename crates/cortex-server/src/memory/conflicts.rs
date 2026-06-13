use cortex_aql::AgentView;
use cortex_engine::Database;

use crate::authz;
use crate::responses::{ConflictIndexResponse, ConflictRecordResponse, RouterError};
use crate::router::query_param_decoded;

pub fn handle_conflicts_shared(
    db: &Database,
    query: &str,
    authenticated_view: Option<&AgentView>,
) -> Result<String, RouterError> {
    let scope = query_param_decoded(query, "scope").map_err(RouterError::BadRequest)?;
    let view = authz::read_view_for_scope(&scope, authenticated_view)?;
    let conflicts = db
        .conflict_index(&view)
        .into_iter()
        .map(|record| ConflictRecordResponse {
            cell_id: record.cell_id.0,
            relation_cell_id: record.relation_cell_id.map(|cell_id| cell_id.0),
            source_cell_id: record.source_cell_id.map(|cell_id| cell_id.0),
            fact: record.fact,
            entity: record.entity,
            metric: record.metric,
            source: record.source,
            source_trust_q16: record.source_trust_q16,
            source_trust_category: record.source_trust_category.as_str().to_owned(),
        })
        .collect::<Vec<_>>();
    let response = ConflictIndexResponse {
        schema_version: "conflict_index.v1",
        scope,
        conflict_count: conflicts.len(),
        conflicts,
    };
    Ok(serde_json::to_string(&response)?)
}
