use crate::audit::{self, AuditAction};
use crate::responses::RouterError;
use crate::ServerOptions;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuthRole {
    Admin,
    Data,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuthTokenPolicy {
    pub token: String,
    pub role: AuthRole,
    pub agent_id: Option<u64>,
}

impl AuthTokenPolicy {
    pub fn new(token: impl Into<String>, role: AuthRole) -> Self {
        Self {
            token: token.into(),
            role,
            agent_id: None,
        }
    }

    pub fn with_agent_id(mut self, agent_id: u64) -> Self {
        self.agent_id = Some(agent_id);
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct AuthDecision {
    pub agent_id: Option<u64>,
}

pub(crate) fn authorize_request(
    options: &ServerOptions,
    auth_header: Option<&str>,
    method: &str,
    path: &str,
) -> Result<AuthDecision, RouterError> {
    let Some(policy) = matching_policy(options, auth_header) else {
        if has_auth(options) {
            return Err(RouterError::Unauthorized);
        }
        return Ok(AuthDecision { agent_id: None });
    };

    if !role_can_access(policy.role, method, path) {
        return Err(RouterError::Forbidden(
            "token role is not allowed to access this route".to_owned(),
        ));
    }
    Ok(AuthDecision {
        agent_id: policy.agent_id,
    })
}

pub(crate) fn validate_token_policies(options: &ServerOptions) -> Result<(), String> {
    for policy in options.effective_auth_tokens() {
        if policy.token.trim().is_empty() {
            return Err("auth token policy contains an empty token".to_owned());
        }
        if matches!(policy.agent_id, Some(0)) {
            return Err("auth token policy agent_id must be greater than zero".to_owned());
        }
    }
    Ok(())
}

fn matching_policy(options: &ServerOptions, auth_header: Option<&str>) -> Option<AuthTokenPolicy> {
    let bearer = auth_header?.strip_prefix("Bearer ")?;
    options
        .effective_auth_tokens()
        .into_iter()
        .find(|policy| policy.token == bearer)
}

fn has_auth(options: &ServerOptions) -> bool {
    options.auth_token.is_some() || !options.auth_tokens.is_empty()
}

fn role_can_access(role: AuthRole, method: &str, path: &str) -> bool {
    match role {
        AuthRole::Admin => true,
        AuthRole::Data => !matches!(route_class(method, path), RouteClass::Admin),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RouteClass {
    Public,
    Data,
    Admin,
}

fn route_class(method: &str, path: &str) -> RouteClass {
    if path == "/" || path == "/dashboard" || path.starts_with("/dashboard/assets/") {
        return RouteClass::Admin;
    }
    match audit::classify(method, path) {
        AuditAction::Health => RouteClass::Public,
        AuditAction::Admin | AuditAction::Metrics => RouteClass::Admin,
        _ => RouteClass::Data,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn data_role_cannot_access_admin_routes() {
        assert!(!role_can_access(AuthRole::Data, "GET", "/v1/stats"));
        assert!(!role_can_access(AuthRole::Data, "POST", "/v1/flush"));
    }

    #[test]
    fn data_role_can_access_data_and_health_routes() {
        assert!(role_can_access(AuthRole::Data, "GET", "/v1/health"));
        assert!(role_can_access(AuthRole::Data, "POST", "/v1/search"));
    }
}
