use super::AuthReviewResponse;

pub(super) fn format_plain(response: &AuthReviewResponse) -> String {
    let mut lines = vec![format!(
        "auth_policy_records={} active_records={} disabled_records={} token_redaction=\"{}\"",
        response.total_records,
        response.active_records,
        response.disabled_records,
        response.token_redaction
    )];
    for record in &response.records {
        lines.push(format!(
            "record source={} line={} principal={} role={} active={} disabled={} agent_id={} quota_per_minute={} body_quota_bytes_per_minute={} queue_quota={} context_budget_tokens={} capabilities={} tenants={} token_redacted={}",
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
            record
                .body_quota_bytes_per_minute
                .map(|value| value.to_string())
                .unwrap_or_else(|| "-".to_owned()),
            record
                .queue_quota
                .map(|value| value.to_string())
                .unwrap_or_else(|| "-".to_owned()),
            record
                .context_budget_tokens
                .map(|value| value.to_string())
                .unwrap_or_else(|| "-".to_owned()),
            record
                .capabilities
                .as_ref()
                .map(|values| values.join(","))
                .unwrap_or_else(|| "-".to_owned()),
            record
                .tenants
                .as_ref()
                .map(|values| values.join(","))
                .unwrap_or_else(|| "-".to_owned()),
            record.token_redacted,
        ));
    }
    lines.join("\n")
}
