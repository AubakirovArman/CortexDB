use cortex_aql::{AgentId, AgentView};
use cortex_core::CellDescriptor;
use cortex_engine::{scope_id, Database};

use crate::context::view_for_scope;
use crate::responses::RouterError;

const READ_DENIED: &str = "scope is not readable by authenticated agent";
const WRITE_DENIED: &str = "scope is not writable by authenticated agent";

pub(crate) fn load_agent_view(
    db: &Database,
    agent_id: Option<AgentId>,
) -> Result<Option<AgentView>, RouterError> {
    let Some(agent_id) = agent_id else {
        return Ok(None);
    };
    db.load_agent_view(agent_id)?.map(Some).ok_or_else(|| {
        RouterError::PermissionDenied("authenticated agent view not found".to_owned())
    })
}

pub(crate) fn read_view_for_scope(
    scope: &str,
    authenticated: Option<&AgentView>,
) -> Result<AgentView, RouterError> {
    match authenticated {
        Some(view) => {
            require_read_scope(view, scope)?;
            Ok(view.clone())
        }
        None => Ok(view_for_scope(scope)),
    }
}

pub(crate) fn verify_view_for_scope(
    scope: &str,
    authenticated: Option<&AgentView>,
) -> Result<AgentView, RouterError> {
    match authenticated {
        Some(view) => {
            if !view.allow_verify_fact {
                return Err(RouterError::PermissionDenied(
                    "verify fact is not allowed by authenticated agent".to_owned(),
                ));
            }
            require_read_scope(view, scope)?;
            Ok(view.clone())
        }
        None => {
            let mut view = view_for_scope(scope);
            view.allow_verify_fact = true;
            Ok(view)
        }
    }
}

pub(crate) fn remember_view_for_scope(
    scope: &str,
    authenticated: Option<&AgentView>,
) -> Result<AgentView, RouterError> {
    match authenticated {
        Some(view) => {
            if !view.allow_remember {
                return Err(RouterError::PermissionDenied(
                    "remember is not allowed by authenticated agent".to_owned(),
                ));
            }
            require_write_scope(view, scope)?;
            Ok(view.clone())
        }
        None => {
            let mut view = view_for_scope(scope);
            view.allow_remember = true;
            view.writable_scopes = std::collections::BTreeSet::from([scope_id(scope)]);
            Ok(view)
        }
    }
}

pub(crate) fn require_descriptor_read(
    authenticated: Option<&AgentView>,
    descriptor: &CellDescriptor,
) -> Result<(), RouterError> {
    if let Some(view) = authenticated {
        require_read_scope(view, &descriptor.scope)?;
    }
    Ok(())
}

pub(crate) fn require_descriptor_write(
    authenticated: Option<&AgentView>,
    descriptor: &CellDescriptor,
) -> Result<(), RouterError> {
    if let Some(view) = authenticated {
        require_write_scope(view, &descriptor.scope)?;
    }
    Ok(())
}

pub(crate) fn require_read_scope(view: &AgentView, scope: &str) -> Result<(), RouterError> {
    if view.can_read_scope(scope_id(scope)) {
        Ok(())
    } else {
        Err(RouterError::PermissionDenied(READ_DENIED.to_owned()))
    }
}

pub(crate) fn require_write_scope(view: &AgentView, scope: &str) -> Result<(), RouterError> {
    if view.can_write_scope(scope_id(scope)) {
        Ok(())
    } else {
        Err(RouterError::PermissionDenied(WRITE_DENIED.to_owned()))
    }
}

pub(crate) fn require_write_scope_for_optional_view(
    view: Option<&AgentView>,
    scope: &str,
) -> Result<(), RouterError> {
    if let Some(view) = view {
        require_write_scope(view, scope)?;
    }
    Ok(())
}
