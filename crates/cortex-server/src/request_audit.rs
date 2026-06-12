use axum::http::StatusCode;
use std::time::Instant;

use crate::responses::ErrorCode;
use crate::router::query_param_opt_decoded;
use crate::state::AppState;
use crate::{audit, llm};

pub(crate) fn audit_http_response(
    state: &AppState,
    event: &RequestAudit<'_>,
    status: StatusCode,
    error_code: Option<ErrorCode>,
) {
    if !state.options.audit_log_enabled {
        return;
    }
    let tenant =
        query_param_opt_decoded(event.query, "tenant").unwrap_or_else(|| "default".to_owned());
    audit::emit_http_response(
        audit::HttpResponseAudit {
            method: event.method,
            path: event.path,
            tenant: &tenant,
            request_id: event.request_id,
            principal_id: event.principal_id.as_deref(),
            auth_role: event.auth_role,
            auth_agent_id: event.auth_agent_id,
            status: status.as_u16(),
            error_code: error_code.map(ErrorCode::as_str),
            duration_ms: event.started.elapsed().as_millis() as u64,
        },
        state.audit_sink.as_deref(),
    );
}

pub(crate) fn audit_llm_inference_decision(
    state: &AppState,
    event: &RequestAudit<'_>,
    status: StatusCode,
    error_code: Option<ErrorCode>,
    decision: &llm::LlmInferenceDecisionAudit,
) {
    if !state.options.audit_log_enabled {
        return;
    }
    let tenant =
        query_param_opt_decoded(event.query, "tenant").unwrap_or_else(|| "default".to_owned());
    audit::emit_llm_inference_decision(
        audit::LlmInferenceDecisionAudit {
            tenant: &tenant,
            request_id: event.request_id,
            principal_id: event.principal_id.as_deref(),
            auth_role: event.auth_role,
            auth_agent_id: event.auth_agent_id,
            status: status.as_u16(),
            error_code: error_code.map(ErrorCode::as_str),
            duration_ms: event.started.elapsed().as_millis() as u64,
            outcome: decision.outcome.as_str(),
            reason: decision.reason,
            provider: decision.provider,
            model: decision.model,
            context_cell_count: decision.context_cell_count,
            citation_count: decision.citation_count,
            request_api_key_present: decision.request_api_key_present,
        },
        state.audit_sink.as_deref(),
    );
}

pub(crate) struct RequestAudit<'a> {
    pub(crate) method: &'a str,
    pub(crate) path: &'a str,
    pub(crate) query: &'a str,
    pub(crate) request_id: &'a str,
    pub(crate) started: Instant,
    pub(crate) principal_id: Option<String>,
    pub(crate) auth_role: Option<&'static str>,
    pub(crate) auth_agent_id: Option<u64>,
}
