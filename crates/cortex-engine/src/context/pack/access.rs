use cortex_aql::AgentView;
use cortex_core::CellId;

use crate::context::{ContextAccessDecision, ContextAccessDecisionOutcome};
use crate::plan::PolicyRewrite;
use crate::query::{scope_id, CellMetadata};

pub(super) fn context_access_decision(
    cell_id: CellId,
    metadata: &CellMetadata,
    view: Option<&AgentView>,
) -> ContextAccessDecision {
    let scope_id = scope_id(&metadata.scope);
    match view {
        Some(view) if PolicyRewrite::allows_scope(view, scope_id) => ContextAccessDecision {
            cell_id,
            decision: ContextAccessDecisionOutcome::Allowed,
            policy: "agent_view_readable_scope".to_owned(),
            reason: "cell scope was present in AgentView.readable_scopes before ContextPack packing"
                .to_owned(),
            scope: metadata.scope.clone(),
            scope_id: scope_id.0,
            agent_id: Some(view.agent_id.0),
        },
        Some(view) => ContextAccessDecision {
            cell_id,
            decision: ContextAccessDecisionOutcome::NotRecorded,
            policy: "agent_view_readable_scope".to_owned(),
            reason: "cell was packed without a positive readable-scope decision; this indicates a policy accounting gap"
                .to_owned(),
            scope: metadata.scope.clone(),
            scope_id: scope_id.0,
            agent_id: Some(view.agent_id.0),
        },
        None => ContextAccessDecision {
            cell_id,
            decision: ContextAccessDecisionOutcome::NotRecorded,
            policy: "no_agent_view".to_owned(),
            reason: "ContextPack was built without an AgentView, so per-cell RBAC attribution was not recorded"
                .to_owned(),
            scope: metadata.scope.clone(),
            scope_id: scope_id.0,
            agent_id: None,
        },
    }
}
