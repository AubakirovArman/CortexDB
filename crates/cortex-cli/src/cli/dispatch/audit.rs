use crate::{cli_audit, cli_audit_siem, cli_auth_review};

use super::DispatchContext;

pub(super) struct AuditReviewDispatch<'a> {
    pub(super) ctx: DispatchContext<'a>,
    pub(super) path: &'a str,
    pub(super) route: Option<String>,
    pub(super) status: Option<u16>,
    pub(super) action: Option<String>,
    pub(super) tenant_filter: Option<String>,
    pub(super) summary_only: bool,
    pub(super) redaction_check: bool,
    pub(super) verify_chain: bool,
    pub(super) mac_key_file: Option<String>,
}

pub(super) fn review(input: AuditReviewDispatch<'_>) -> Result<String, String> {
    let mac_key = load_mac_key(input.mac_key_file.as_deref())?;
    cli_audit::review(cli_audit::AuditReviewOptions {
        path: input.path,
        route: input.route.as_deref(),
        status: input.status,
        action: input.action.as_deref(),
        tenant: input.tenant_filter.as_deref(),
        summary_only: input.summary_only,
        redaction_check: input.redaction_check,
        verify_chain: input.verify_chain,
        mac_key: mac_key.as_ref(),
        json: input.ctx.json,
    })
}

pub(super) fn export_siem(
    ctx: DispatchContext<'_>,
    input_path: String,
    output_path: String,
    redaction_check: bool,
    verify_chain: bool,
    mac_key_file: Option<String>,
) -> Result<String, String> {
    let mac_key = load_mac_key(mac_key_file.as_deref())?;
    cli_audit_siem::export_jsonl(
        &input_path,
        &output_path,
        redaction_check,
        verify_chain,
        mac_key.as_ref(),
        ctx.json,
    )
}

pub(super) fn auth_review(
    ctx: DispatchContext<'_>,
    policy_store: Option<String>,
    tokens_file: Option<String>,
    tokens: Option<String>,
    tokens_env: Option<String>,
) -> Result<String, String> {
    cli_auth_review::review(cli_auth_review::AuthReviewOptions {
        policy_store: policy_store.as_deref(),
        tokens_file: tokens_file.as_deref(),
        tokens: tokens.as_deref(),
        tokens_env: tokens_env.as_deref(),
        json: ctx.json,
    })
}

fn load_mac_key(path: Option<&str>) -> Result<Option<cortex_crypto::MacKey>, String> {
    let Some(path) = path else {
        return Ok(None);
    };
    let raw = std::fs::read_to_string(path)
        .map_err(|error| format!("audit MAC key file could not be read: {error}"))?;
    crate::cli_audit_chain::mac_key_from_hex(raw.trim()).map(Some)
}
