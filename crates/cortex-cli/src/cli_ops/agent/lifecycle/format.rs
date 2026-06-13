use cortex_aql::{AgentView, MemoryType, RetrievalMode};
use serde::Serialize;

#[derive(Serialize)]
struct AgentViewCliResponse {
    schema_version: &'static str,
    agent_id: u64,
    label: Option<String>,
    readable_brains: Vec<u64>,
    readable_scopes: Vec<u64>,
    writable_scopes: Vec<u64>,
    allowed_modes: Vec<&'static str>,
    allowed_memory_types: Vec<&'static str>,
    max_context_budget_tokens: u32,
    default_context_budget_tokens: u32,
    max_candidate_limit: u32,
    default_candidate_limit: u32,
    min_required_confidence_q16: u16,
    max_ttl_seconds: Option<u64>,
    allow_remember: bool,
    allow_verify_fact: bool,
    allow_audit_mode: bool,
    require_citations_by_default: bool,
    private_scope: Option<u64>,
}

#[derive(Serialize)]
struct AgentViewListCliResponse {
    schema_version: &'static str,
    agents: Vec<AgentViewCliResponse>,
}

#[derive(Serialize)]
struct AgentScopeMutationCliResponse<'a> {
    schema_version: &'static str,
    action: &'static str,
    agent_id: u64,
    scope: &'a str,
    access: &'static str,
    readable_scope_count: usize,
    writable_scope_count: usize,
}

pub(super) fn format_agent_response(json_output: bool, view: &AgentView) -> Result<String, String> {
    if json_output {
        json(&agent_response(view))
    } else {
        Ok(format_agent_line(view))
    }
}

pub(super) fn format_agent_list(json_output: bool, views: &[AgentView]) -> Result<String, String> {
    if json_output {
        let response = AgentViewListCliResponse {
            schema_version: "cortexdb.cli.agent_views.v1",
            agents: views.iter().map(agent_response).collect(),
        };
        return json(&response);
    }
    if views.is_empty() {
        return Ok("agents=0".to_owned());
    }
    Ok(views
        .iter()
        .map(format_agent_line)
        .collect::<Vec<_>>()
        .join("\n"))
}

pub(super) fn format_scope_mutation(
    json_output: bool,
    action: &'static str,
    agent_id: u64,
    scope: &str,
    access: &'static str,
    readable_scope_count: usize,
    writable_scope_count: usize,
) -> Result<String, String> {
    let response = AgentScopeMutationCliResponse {
        schema_version: "cortexdb.cli.agent_scope_mutation.v1",
        action,
        agent_id,
        scope,
        access,
        readable_scope_count,
        writable_scope_count,
    };
    if json_output {
        json(&response)
    } else {
        Ok(format!(
            "agent_id={} action={} scope={} access={} readable_scope_count={} writable_scope_count={}",
            response.agent_id,
            response.action,
            response.scope,
            response.access,
            response.readable_scope_count,
            response.writable_scope_count
        ))
    }
}

fn format_agent_line(view: &AgentView) -> String {
    format!(
        "agent_id={} label={} readable_brains={} readable_scopes={} writable_scopes={} modes={} memory_types={} max_context_budget_tokens={} default_context_budget_tokens={} max_candidate_limit={} default_candidate_limit={}",
        view.agent_id.0,
        view.label.as_deref().unwrap_or(""),
        join_u64(view.readable_brains.iter().map(|value| value.0)),
        join_u64(view.readable_scopes.iter().map(|value| value.0)),
        join_u64(view.writable_scopes.iter().map(|value| value.0)),
        view.allowed_modes.iter().map(mode_label).collect::<Vec<_>>().join(","),
        view.allowed_memory_types
            .iter()
            .map(memory_type_label)
            .collect::<Vec<_>>()
            .join(","),
        view.max_context_budget_tokens,
        view.default_context_budget_tokens,
        view.max_candidate_limit,
        view.default_candidate_limit
    )
}

fn agent_response(view: &AgentView) -> AgentViewCliResponse {
    AgentViewCliResponse {
        schema_version: "cortexdb.cli.agent_view.v1",
        agent_id: view.agent_id.0,
        label: view.label.clone(),
        readable_brains: view.readable_brains.iter().map(|value| value.0).collect(),
        readable_scopes: view.readable_scopes.iter().map(|value| value.0).collect(),
        writable_scopes: view.writable_scopes.iter().map(|value| value.0).collect(),
        allowed_modes: view.allowed_modes.iter().map(mode_label).collect(),
        allowed_memory_types: view
            .allowed_memory_types
            .iter()
            .map(memory_type_label)
            .collect(),
        max_context_budget_tokens: view.max_context_budget_tokens,
        default_context_budget_tokens: view.default_context_budget_tokens,
        max_candidate_limit: view.max_candidate_limit,
        default_candidate_limit: view.default_candidate_limit,
        min_required_confidence_q16: view.min_required_confidence_q16,
        max_ttl_seconds: view.max_ttl_seconds,
        allow_remember: view.allow_remember,
        allow_verify_fact: view.allow_verify_fact,
        allow_audit_mode: view.allow_audit_mode,
        require_citations_by_default: view.require_citations_by_default,
        private_scope: view.private_scope.map(|scope| scope.0),
    }
}

fn json<T: Serialize>(value: &T) -> Result<String, String> {
    serde_json::to_string(value).map_err(|error| error.to_string())
}

fn join_u64(values: impl Iterator<Item = u64>) -> String {
    values
        .map(|value| value.to_string())
        .collect::<Vec<_>>()
        .join(",")
}

fn mode_label(mode: &RetrievalMode) -> &'static str {
    match mode {
        RetrievalMode::Fast => "fast",
        RetrievalMode::Balanced => "balanced",
        RetrievalMode::Semantic => "semantic",
        RetrievalMode::Audit => "audit",
    }
}

fn memory_type_label(memory_type: &MemoryType) -> &'static str {
    match memory_type {
        MemoryType::Decision => "decision",
        MemoryType::Preference => "preference",
        MemoryType::WorkflowResult => "workflow_result",
        MemoryType::ErrorLog => "error_log",
        MemoryType::Observation => "observation",
    }
}
