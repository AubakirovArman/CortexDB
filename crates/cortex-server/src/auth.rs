use crate::audit::{self, AuditAction};
use crate::responses::RouterError;
use crate::ServerOptions;
use std::path::Path;

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
    let Some(policy) = matching_policy(options, auth_header)? else {
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
    for policy in effective_auth_tokens(options)? {
        if policy.token.trim().is_empty() {
            return Err("auth token policy contains an empty token".to_owned());
        }
        if matches!(policy.agent_id, Some(0)) {
            return Err("auth token policy agent_id must be greater than zero".to_owned());
        }
    }
    Ok(())
}

pub fn parse_auth_tokens(raw: &str) -> Result<Vec<AuthTokenPolicy>, String> {
    let mut tokens = Vec::new();
    for entry in raw
        .split(',')
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
    {
        tokens.push(parse_auth_token_policy(entry)?);
    }
    if tokens.is_empty() {
        return Err("auth token list must contain at least one token policy".to_owned());
    }
    Ok(tokens)
}

fn parse_auth_token_policy(raw: &str) -> Result<AuthTokenPolicy, String> {
    let parts = raw.split(':').collect::<Vec<_>>();
    if !(2..=3).contains(&parts.len()) {
        return Err("auth token entries must use role:token or role:token:agent_id".to_owned());
    }
    let role = match parts[0].trim().to_ascii_lowercase().as_str() {
        "admin" => AuthRole::Admin,
        "data" => AuthRole::Data,
        _ => return Err("auth token role must be admin or data".to_owned()),
    };
    let token = parts[1].trim();
    if token.is_empty() {
        return Err("auth token must not be empty".to_owned());
    }
    let mut policy = AuthTokenPolicy::new(token.to_owned(), role);
    if parts.len() == 3 {
        let agent_id = parts[2]
            .trim()
            .parse::<u64>()
            .map_err(|_| "auth token agent_id must be a positive integer".to_owned())?;
        if agent_id == 0 {
            return Err("auth token agent_id must be greater than zero".to_owned());
        }
        policy = policy.with_agent_id(agent_id);
    }
    Ok(policy)
}

fn load_auth_tokens_file(path: &Path) -> Result<Vec<AuthTokenPolicy>, String> {
    let raw = std::fs::read_to_string(path)
        .map_err(|error| format!("auth token policy file could not be read: {error}"))?;
    let mut tokens = Vec::new();
    for (index, line) in raw.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        tokens.push(parse_auth_token_policy(trimmed).map_err(|error| {
            format!(
                "auth token policy file line {} is invalid: {error}",
                index + 1
            )
        })?);
    }
    if tokens.is_empty() {
        return Err("auth token policy file must contain at least one token policy".to_owned());
    }
    Ok(tokens)
}

fn effective_auth_tokens(options: &ServerOptions) -> Result<Vec<AuthTokenPolicy>, String> {
    let mut tokens = options.effective_auth_tokens();
    if let Some(path) = &options.auth_tokens_file {
        tokens.extend(load_auth_tokens_file(path)?);
    }
    Ok(tokens)
}

fn matching_policy(
    options: &ServerOptions,
    auth_header: Option<&str>,
) -> Result<Option<AuthTokenPolicy>, RouterError> {
    let Some(bearer) = auth_header.and_then(|value| value.strip_prefix("Bearer ")) else {
        return Ok(None);
    };
    let tokens = effective_auth_tokens(options).map_err(|_| {
        RouterError::Internal("auth token policy file could not be read".to_owned())
    })?;
    Ok(tokens.into_iter().find(|policy| policy.token == bearer))
}

fn has_auth(options: &ServerOptions) -> bool {
    options.auth_token.is_some()
        || !options.auth_tokens.is_empty()
        || options.auth_tokens_file.is_some()
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

    #[test]
    fn parse_auth_tokens_accepts_role_token_agent_entries() {
        let tokens = parse_auth_tokens("admin:root,data:worker:7").unwrap();
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0].role, AuthRole::Admin);
        assert_eq!(tokens[0].token, "root");
        assert_eq!(tokens[0].agent_id, None);
        assert_eq!(tokens[1].role, AuthRole::Data);
        assert_eq!(tokens[1].token, "worker");
        assert_eq!(tokens[1].agent_id, Some(7));
    }

    #[test]
    fn parse_auth_tokens_rejects_invalid_entries() {
        assert!(parse_auth_tokens("").is_err());
        assert!(parse_auth_tokens("root").is_err());
        assert!(parse_auth_tokens("admin:").is_err());
        assert!(parse_auth_tokens("owner:root").is_err());
        assert!(parse_auth_tokens("data:worker:0").is_err());
    }

    #[test]
    fn token_policy_file_allows_comments_and_blank_lines() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("auth.tokens");
        std::fs::write(&path, "# comment\n\nadmin:root\ndata:worker:9\n").unwrap();
        let tokens = load_auth_tokens_file(&path).unwrap();
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0].role, AuthRole::Admin);
        assert_eq!(tokens[1].agent_id, Some(9));
    }
}
