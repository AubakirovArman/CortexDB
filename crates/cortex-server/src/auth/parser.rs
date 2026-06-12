use super::{AuthRole, AuthTokenPolicy};
use crate::auth_capability::EffectiveAuthPolicy;
use crate::auth_policy_store;
use crate::ServerOptions;
use std::path::Path;

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

pub(super) fn effective_auth_policies(
    options: &ServerOptions,
) -> Result<Vec<EffectiveAuthPolicy>, String> {
    let mut tokens = options
        .effective_auth_tokens()
        .into_iter()
        .map(EffectiveAuthPolicy::from_token_policy)
        .collect::<Vec<_>>();
    if let Some(path) = &options.auth_tokens_file {
        tokens.extend(
            load_auth_tokens_file(path)?
                .into_iter()
                .map(EffectiveAuthPolicy::from_token_policy),
        );
    }
    if let Some(path) = &options.auth_policy_store_file {
        tokens.extend(load_auth_policy_store(path)?);
    }
    Ok(tokens)
}

pub(super) fn parse_role(raw: &str) -> Result<AuthRole, String> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "admin" => Ok(AuthRole::Admin),
        "data" => Ok(AuthRole::Data),
        _ => Err("auth token role must be admin or data".to_owned()),
    }
}

pub(crate) fn load_auth_tokens_file(path: &Path) -> Result<Vec<AuthTokenPolicy>, String> {
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

pub(crate) fn load_auth_policy_store(path: &Path) -> Result<Vec<EffectiveAuthPolicy>, String> {
    auth_policy_store::load_token_policies_from_store(path)
}

fn parse_auth_token_policy(raw: &str) -> Result<AuthTokenPolicy, String> {
    let parts = raw.split(':').collect::<Vec<_>>();
    if !(2..=3).contains(&parts.len()) {
        return Err("auth token entries must use role:token or role:token:agent_id".to_owned());
    }
    let role = parse_role(parts[0])?;
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
