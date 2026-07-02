use cortex_aql::AgentView;
use cortex_core::CellId;

use crate::access_capture::{agent_view_digest, CAPTURED_ACCESS_POLICY_VERSION};
use crate::context::{ContextAccessDecision, ContextAccessDecisionOutcome};
use crate::database::CapturedAccessDecision;
use crate::plan::PolicyRewrite;
use crate::query::{scope_id, CellMetadata};

pub(super) fn context_access_decision(
    cell_id: CellId,
    metadata: &CellMetadata,
    captured: Option<&CapturedAccessDecision>,
    view: Option<&AgentView>,
) -> ContextAccessDecision {
    if let Some(captured) = captured {
        return ContextAccessDecision {
            cell_id: captured.cell_id,
            decision: ContextAccessDecisionOutcome::Allowed,
            policy: captured.policy.clone(),
            policy_version: Some(captured.policy_version.clone()),
            reason: captured.reason.clone(),
            scope: captured.scope.clone(),
            scope_id: captured.scope_id,
            agent_id: captured.agent_id,
            agent_view_digest: Some(captured.agent_view_digest.clone()),
        };
    }

    let scope_id = scope_id(&metadata.scope);
    match view {
        Some(view) if PolicyRewrite::allows_scope(view, scope_id) => ContextAccessDecision {
            cell_id,
            decision: ContextAccessDecisionOutcome::Allowed,
            policy: "agent_view_readable_scope".to_owned(),
            policy_version: Some(CAPTURED_ACCESS_POLICY_VERSION.to_owned()),
            reason:
                "cell scope was re-derived from AgentView.readable_scopes during ContextPack packing"
                    .to_owned(),
            scope: metadata.scope.clone(),
            scope_id: scope_id.0,
            agent_id: Some(view.agent_id.0),
            agent_view_digest: Some(agent_view_digest(view)),
        },
        Some(view) => ContextAccessDecision {
            cell_id,
            decision: ContextAccessDecisionOutcome::NotRecorded,
            policy: "agent_view_readable_scope".to_owned(),
            policy_version: Some(CAPTURED_ACCESS_POLICY_VERSION.to_owned()),
            reason: "cell was packed without a positive readable-scope decision; this indicates a policy accounting gap"
                .to_owned(),
            scope: metadata.scope.clone(),
            scope_id: scope_id.0,
            agent_id: Some(view.agent_id.0),
            agent_view_digest: Some(agent_view_digest(view)),
        },
        None => ContextAccessDecision {
            cell_id,
            decision: ContextAccessDecisionOutcome::NotRecorded,
            policy: "no_agent_view".to_owned(),
            policy_version: None,
            reason: "ContextPack was built without an AgentView, so per-cell RBAC attribution was not recorded"
                .to_owned(),
            scope: metadata.scope.clone(),
            scope_id: scope_id.0,
            agent_id: None,
            agent_view_digest: None,
        },
    }
}
