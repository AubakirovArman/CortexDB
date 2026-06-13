use std::collections::BTreeSet;

pub(super) fn validate_principal(principal_id: &str, line: usize) -> Result<(), String> {
    if principal_id.is_empty() {
        return Err(format!(
            "auth policy store principal {line} has empty principal_id"
        ));
    }
    Ok(())
}

pub(super) fn validate_role(role: &str) -> Result<(), String> {
    match role.trim().to_ascii_lowercase().as_str() {
        "admin" | "data" => Ok(()),
        _ => Err("auth token role must be admin or data".to_owned()),
    }
}

pub(super) fn validate_agent_id(agent_id: Option<u64>) -> Result<(), String> {
    if matches!(agent_id, Some(0)) {
        return Err("auth token policy agent_id must be greater than zero".to_owned());
    }
    Ok(())
}

pub(super) fn validate_u64_quota(quota: Option<u64>, field: &str) -> Result<(), String> {
    if matches!(quota, Some(0)) {
        return Err(format!(
            "auth token policy {field} must be greater than zero"
        ));
    }
    Ok(())
}

pub(super) fn validate_u32_quota(quota: Option<u32>, field: &str) -> Result<(), String> {
    if matches!(quota, Some(0)) {
        return Err(format!(
            "auth token policy {field} must be greater than zero"
        ));
    }
    Ok(())
}

pub(super) fn validate_capabilities(
    raw: Option<Vec<String>>,
) -> Result<Option<Vec<String>>, String> {
    let Some(values) = raw else {
        return Ok(None);
    };
    if values.is_empty() {
        return Err("auth policy capabilities must not be empty".to_owned());
    }
    let mut seen = BTreeSet::new();
    let mut normalized = Vec::new();
    for value in values {
        let capability = value.trim().to_ascii_lowercase();
        match capability.as_str() {
            "admin" | "aql" | "context" | "delete" | "ingest" | "inference" | "memory"
            | "metrics" | "read" | "search" | "verify" | "write" => {}
            _ => return Err("auth policy capability is not recognized".to_owned()),
        }
        if !seen.insert(capability.clone()) {
            return Err("auth policy capability is duplicated".to_owned());
        }
        normalized.push(capability);
    }
    Ok(Some(normalized))
}

pub(super) fn validate_tenants(raw: Option<Vec<String>>) -> Result<Option<Vec<String>>, String> {
    let Some(values) = raw else {
        return Ok(None);
    };
    if values.is_empty() {
        return Err("auth policy tenants must not be empty".to_owned());
    }
    let mut seen = BTreeSet::new();
    let mut normalized = Vec::new();
    for value in values {
        let tenant = value.trim();
        if !validate_tenant_id(tenant) {
            return Err("auth policy tenant is invalid".to_owned());
        }
        if !seen.insert(tenant.to_owned()) {
            return Err("auth policy tenant is duplicated".to_owned());
        }
        normalized.push(tenant.to_owned());
    }
    Ok(Some(normalized))
}

pub(super) fn validate_tenant_id(tenant: &str) -> bool {
    if tenant.is_empty() || tenant.len() > 64 {
        return false;
    }
    tenant
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}
