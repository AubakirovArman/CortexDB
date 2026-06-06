use cortex_aql::AgentId;
use cortex_engine::{scope_id, Database};
use serde::{Deserialize, Serialize};

use crate::responses::RouterError;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AgentScopeAccess {
    Read,
    Write,
    ReadWrite,
}

impl AgentScopeAccess {
    fn parse(raw: &str) -> Result<Self, RouterError> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "read" => Ok(Self::Read),
            "write" => Ok(Self::Write),
            "read_write" | "read-write" | "both" => Ok(Self::ReadWrite),
            _ => Err(RouterError::BadRequest(
                "scope access must be read, write, or read_write".to_owned(),
            )),
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::Write => "write",
            Self::ReadWrite => "read_write",
        }
    }
}

#[derive(Debug, Deserialize)]
struct AgentScopeMutationRequest {
    agent_id: u64,
    scope: String,
    access: String,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct AgentScopeMutationResponse {
    pub schema_version: &'static str,
    pub action: &'static str,
    pub agent_id: u64,
    pub scope: String,
    pub access: &'static str,
    pub readable_scope_count: usize,
    pub writable_scope_count: usize,
}

pub(crate) fn parse_scope_mutation_body(
    body: &[u8],
) -> Result<(AgentId, String, AgentScopeAccess), RouterError> {
    let request = serde_json::from_slice::<AgentScopeMutationRequest>(body).map_err(|error| {
        RouterError::BadRequest(format!("invalid auth scope mutation JSON: {error}"))
    })?;
    if request.agent_id == 0 {
        return Err(RouterError::BadRequest(
            "agent_id must be greater than zero".to_owned(),
        ));
    }
    let scope = validate_scope_label(&request.scope)?;
    let access = AgentScopeAccess::parse(&request.access)?;
    Ok((AgentId(request.agent_id), scope, access))
}

pub(crate) fn apply_agent_scope_mutation(
    db: &mut Database,
    agent_id: AgentId,
    scope: &str,
    access: AgentScopeAccess,
    grant: bool,
) -> Result<AgentScopeMutationResponse, RouterError> {
    let mut view = db
        .load_agent_view(agent_id)
        .map_err(|error| RouterError::Internal(error.to_string()))?
        .ok_or_else(|| RouterError::NotFound("authenticated agent view not found".to_owned()))?;
    let scope_id = scope_id(scope);
    if grant {
        if matches!(access, AgentScopeAccess::Read | AgentScopeAccess::ReadWrite) {
            view.readable_scopes.insert(scope_id);
        }
        if matches!(
            access,
            AgentScopeAccess::Write | AgentScopeAccess::ReadWrite
        ) {
            view.writable_scopes.insert(scope_id);
        }
    } else {
        if matches!(access, AgentScopeAccess::Read | AgentScopeAccess::ReadWrite) {
            view.readable_scopes.remove(&scope_id);
        }
        if matches!(
            access,
            AgentScopeAccess::Write | AgentScopeAccess::ReadWrite
        ) {
            view.writable_scopes.remove(&scope_id);
        }
    }
    db.save_agent_view(&view)
        .map_err(|error| RouterError::Internal(error.to_string()))?;

    Ok(AgentScopeMutationResponse {
        schema_version: "cortexdb.auth_scope_mutation.v1",
        action: if grant { "grant_scope" } else { "revoke_scope" },
        agent_id: agent_id.0,
        scope: scope.to_owned(),
        access: access.as_str(),
        readable_scope_count: view.readable_scopes.len(),
        writable_scope_count: view.writable_scopes.len(),
    })
}

fn validate_scope_label(scope: &str) -> Result<String, RouterError> {
    let scope = scope.trim();
    if scope.is_empty() {
        return Err(RouterError::BadRequest(
            "scope must not be empty".to_owned(),
        ));
    }
    if scope.chars().any(char::is_control) {
        return Err(RouterError::BadRequest(
            "scope must not contain control characters".to_owned(),
        ));
    }
    Ok(scope.to_owned())
}
