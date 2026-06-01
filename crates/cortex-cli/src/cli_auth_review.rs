use std::collections::BTreeSet;
use std::fs;

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
struct AuthReviewResponse {
    schema_version: &'static str,
    total_records: usize,
    active_records: usize,
    disabled_records: usize,
    token_redaction: &'static str,
    records: Vec<AuthReviewRecord>,
}

#[derive(Debug, Serialize)]
struct AuthReviewRecord {
    source: String,
    source_line: Option<usize>,
    principal_id: Option<String>,
    role: String,
    agent_id: Option<u64>,
    request_quota_per_minute: Option<u64>,
    disabled: bool,
    active: bool,
    token_present: bool,
    token_redacted: bool,
}

#[derive(Debug)]
pub(crate) struct AuthReviewOptions<'a> {
    pub policy_store: Option<&'a str>,
    pub tokens_file: Option<&'a str>,
    pub tokens: Option<&'a str>,
    pub json: bool,
}

#[derive(Deserialize)]
struct AuthPolicyStoreFile {
    schema_version: String,
    principals: Vec<AuthPolicyPrincipal>,
}

#[derive(Deserialize)]
struct AuthPolicyPrincipal {
    principal_id: String,
    token: String,
    role: String,
    #[serde(default)]
    agent_id: Option<u64>,
    #[serde(default)]
    disabled: bool,
    #[serde(default)]
    request_quota_per_minute: Option<u64>,
}

pub(crate) fn review(options: AuthReviewOptions<'_>) -> Result<String, String> {
    let mut records = Vec::new();
    if let Some(path) = options.policy_store {
        records.extend(load_policy_store(path)?);
    }
    if let Some(path) = options.tokens_file {
        records.extend(load_tokens_file(path)?);
    }
    if let Some(tokens) = options.tokens {
        records.extend(parse_inline_tokens(tokens)?);
    }
    if records.is_empty() {
        return Err("auth review needs --policy-store, --tokens-file, or --tokens".to_owned());
    }

    let response = AuthReviewResponse {
        schema_version: "cortexdb.auth_review.v1",
        total_records: records.len(),
        active_records: records.iter().filter(|record| record.active).count(),
        disabled_records: records.iter().filter(|record| record.disabled).count(),
        token_redaction: "tokens are never printed",
        records,
    };
    if options.json {
        serde_json::to_string_pretty(&response).map_err(|error| error.to_string())
    } else {
        Ok(format_plain(&response))
    }
}

fn load_policy_store(path: &str) -> Result<Vec<AuthReviewRecord>, String> {
    let raw = fs::read_to_string(path)
        .map_err(|error| format!("auth policy store could not be read: {error}"))?;
    let store = serde_json::from_str::<AuthPolicyStoreFile>(&raw)
        .map_err(|error| format!("auth policy store is invalid JSON: {error}"))?;
    if store.schema_version != "cortexdb.auth_policy.v1" {
        return Err("auth policy store schema_version must be cortexdb.auth_policy.v1".to_owned());
    }

    let mut seen_principals = BTreeSet::new();
    let mut seen_tokens = BTreeSet::new();
    let mut records = Vec::new();
    for (index, principal) in store.principals.into_iter().enumerate() {
        let principal_id = principal.principal_id.trim().to_owned();
        validate_principal(&principal_id, index + 1)?;
        if !seen_principals.insert(principal_id.clone()) {
            return Err(format!(
                "auth policy store principal_id {principal_id:?} is duplicated"
            ));
        }
        let token = principal.token.trim();
        if token.is_empty() {
            return Err(format!(
                "auth policy store principal {principal_id:?} has empty token"
            ));
        }
        if !seen_tokens.insert(token.to_owned()) {
            return Err(format!(
                "auth policy store token for principal {principal_id:?} is duplicated"
            ));
        }
        validate_role(&principal.role)?;
        validate_agent_id(principal.agent_id)?;
        validate_quota(principal.request_quota_per_minute)?;
        records.push(AuthReviewRecord {
            source: path.to_owned(),
            source_line: Some(index + 1),
            principal_id: Some(principal_id),
            role: principal.role.trim().to_ascii_lowercase(),
            agent_id: principal.agent_id,
            request_quota_per_minute: principal.request_quota_per_minute,
            disabled: principal.disabled,
            active: !principal.disabled,
            token_present: true,
            token_redacted: true,
        });
    }
    Ok(records)
}

fn load_tokens_file(path: &str) -> Result<Vec<AuthReviewRecord>, String> {
    let raw = fs::read_to_string(path)
        .map_err(|error| format!("auth token policy file could not be read: {error}"))?;
    let mut records = Vec::new();
    for (index, line) in raw.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        records.push(parse_token_entry(
            trimmed,
            path.to_owned(),
            Some(index + 1),
        )?);
    }
    if records.is_empty() {
        return Err("auth token policy file must contain at least one token policy".to_owned());
    }
    Ok(records)
}

fn parse_inline_tokens(raw: &str) -> Result<Vec<AuthReviewRecord>, String> {
    let mut records = Vec::new();
    for (index, entry) in raw
        .split(',')
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
        .enumerate()
    {
        records.push(parse_token_entry(
            entry,
            "inline_tokens".to_owned(),
            Some(index + 1),
        )?);
    }
    if records.is_empty() {
        return Err("auth token list must contain at least one token policy".to_owned());
    }
    Ok(records)
}

fn parse_token_entry(
    raw: &str,
    source: String,
    source_line: Option<usize>,
) -> Result<AuthReviewRecord, String> {
    let parts = raw.split(':').collect::<Vec<_>>();
    if !(2..=3).contains(&parts.len()) {
        return Err("auth token entries must use role:token or role:token:agent_id".to_owned());
    }
    validate_role(parts[0])?;
    let token = parts[1].trim();
    if token.is_empty() {
        return Err("auth token must not be empty".to_owned());
    }
    let agent_id = if parts.len() == 3 {
        let value = parts[2]
            .trim()
            .parse::<u64>()
            .map_err(|_| "auth token agent_id must be a positive integer".to_owned())?;
        validate_agent_id(Some(value))?;
        Some(value)
    } else {
        None
    };
    Ok(AuthReviewRecord {
        source,
        source_line,
        principal_id: None,
        role: parts[0].trim().to_ascii_lowercase(),
        agent_id,
        request_quota_per_minute: None,
        disabled: false,
        active: true,
        token_present: true,
        token_redacted: true,
    })
}

fn validate_principal(principal_id: &str, line: usize) -> Result<(), String> {
    if principal_id.is_empty() {
        return Err(format!(
            "auth policy store principal {line} has empty principal_id"
        ));
    }
    Ok(())
}

fn validate_role(role: &str) -> Result<(), String> {
    match role.trim().to_ascii_lowercase().as_str() {
        "admin" | "data" => Ok(()),
        _ => Err("auth token role must be admin or data".to_owned()),
    }
}

fn validate_agent_id(agent_id: Option<u64>) -> Result<(), String> {
    if matches!(agent_id, Some(0)) {
        return Err("auth token policy agent_id must be greater than zero".to_owned());
    }
    Ok(())
}

fn validate_quota(quota: Option<u64>) -> Result<(), String> {
    if matches!(quota, Some(0)) {
        return Err(
            "auth token policy request_quota_per_minute must be greater than zero".to_owned(),
        );
    }
    Ok(())
}

fn format_plain(response: &AuthReviewResponse) -> String {
    let mut lines = vec![format!(
        "auth_policy_records={} active_records={} disabled_records={} token_redaction=\"{}\"",
        response.total_records,
        response.active_records,
        response.disabled_records,
        response.token_redaction
    )];
    for record in &response.records {
        lines.push(format!(
            "record source={} line={} principal={} role={} active={} disabled={} agent_id={} quota_per_minute={} token_redacted={}",
            record.source,
            record
                .source_line
                .map(|value| value.to_string())
                .unwrap_or_else(|| "-".to_owned()),
            record.principal_id.as_deref().unwrap_or("-"),
            record.role,
            record.active,
            record.disabled,
            record
                .agent_id
                .map(|value| value.to_string())
                .unwrap_or_else(|| "-".to_owned()),
            record
                .request_quota_per_minute
                .map(|value| value.to_string())
                .unwrap_or_else(|| "-".to_owned()),
            record.token_redacted,
        ));
    }
    lines.join("\n")
}
