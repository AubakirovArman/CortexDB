use std::collections::BTreeSet;
use std::str::FromStr;

use cortex_aql::{AgentId, AgentView, BrainId, MemoryType, RetrievalMode, ScopeId, Q16_ZERO};
use cortex_engine::{scope_id, Database};
use serde::{Deserialize, Serialize};

use crate::responses::RouterError;

#[derive(Debug, Deserialize)]
pub(crate) struct AgentViewCreateRequest {
    pub agent_id: u64,
    pub label: Option<String>,
    #[serde(default)]
    pub readable_scopes: Vec<String>,
    #[serde(default)]
    pub writable_scopes: Vec<String>,
    #[serde(default)]
    pub readable_brains: Vec<u64>,
    #[serde(default)]
    pub allowed_modes: Vec<String>,
    #[serde(default)]
    pub allowed_memory_types: Vec<String>,
    pub max_context_budget_tokens: Option<u32>,
    pub default_context_budget_tokens: Option<u32>,
    pub max_candidate_limit: Option<u32>,
    pub default_candidate_limit: Option<u32>,
    pub min_required_confidence_q16: Option<u16>,
    pub max_ttl_seconds: Option<u64>,
    pub allow_remember: Option<bool>,
    pub allow_verify_fact: Option<bool>,
    pub allow_audit_mode: Option<bool>,
    pub require_citations_by_default: Option<bool>,
    pub private_scope: Option<String>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub(crate) struct AgentViewResponse {
    pub schema_version: &'static str,
    pub agent_id: u64,
    pub label: Option<String>,
    pub readable_brains: Vec<u64>,
    pub readable_scopes: Vec<u64>,
    pub writable_scopes: Vec<u64>,
    pub allowed_modes: Vec<&'static str>,
    pub allowed_memory_types: Vec<&'static str>,
    pub max_context_budget_tokens: u32,
    pub default_context_budget_tokens: u32,
    pub max_candidate_limit: u32,
    pub default_candidate_limit: u32,
    pub min_required_confidence_q16: u16,
    pub max_ttl_seconds: Option<u64>,
    pub allow_remember: bool,
    pub allow_verify_fact: bool,
    pub allow_audit_mode: bool,
    pub require_citations_by_default: bool,
    pub private_scope: Option<u64>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub(crate) struct AgentViewListResponse {
    pub schema_version: &'static str,
    pub agents: Vec<AgentViewResponse>,
}

pub(crate) fn parse_create_body(body: &[u8]) -> Result<AgentViewCreateRequest, RouterError> {
    serde_json::from_slice::<AgentViewCreateRequest>(body)
        .map_err(|error| RouterError::BadRequest(format!("invalid agent view JSON: {error}")))
}

pub(crate) fn create_agent_view(
    db: &mut Database,
    request: AgentViewCreateRequest,
) -> Result<AgentViewResponse, RouterError> {
    let view = request.into_agent_view()?;
    db.save_agent_view(&view)
        .map_err(|error| RouterError::Internal(error.to_string()))?;
    Ok(agent_view_response(&view))
}

pub(crate) fn list_agent_views(db: &Database) -> Result<AgentViewListResponse, RouterError> {
    let agents = db
        .list_agent_views()
        .map_err(|error| RouterError::Internal(error.to_string()))?
        .iter()
        .map(agent_view_response)
        .collect();
    Ok(AgentViewListResponse {
        schema_version: "cortexdb.agent_views.v1",
        agents,
    })
}

pub(crate) fn show_agent_view(
    db: &Database,
    agent_id: AgentId,
) -> Result<AgentViewResponse, RouterError> {
    db.load_agent_view(agent_id)
        .map_err(|error| RouterError::Internal(error.to_string()))?
        .map(|view| agent_view_response(&view))
        .ok_or_else(|| RouterError::NotFound("agent view not found".to_owned()))
}

#[allow(dead_code)]
pub(crate) fn handle_sync_request(
    db: &mut Database,
    method: &str,
    path: &str,
    body: &[u8],
) -> Result<Option<String>, RouterError> {
    let response = match (method, path) {
        ("POST", "/v1/agents") => {
            let request = parse_create_body(body)?;
            serde_json::to_string(&create_agent_view(db, request)?)?
        }
        ("GET", "/v1/agents") => serde_json::to_string(&list_agent_views(db)?)?,
        ("GET", _) => {
            let Some(agent_id) = parse_agent_path(path) else {
                return Ok(None);
            };
            serde_json::to_string(&show_agent_view(db, agent_id)?)?
        }
        _ => return Ok(None),
    };
    Ok(Some(response))
}

pub(crate) fn parse_agent_path(path: &str) -> Option<AgentId> {
    let raw = path.strip_prefix("/v1/agents/")?;
    if raw.contains('/') {
        return None;
    }
    raw.parse::<u64>()
        .ok()
        .filter(|value| *value > 0)
        .map(AgentId)
}

impl AgentViewCreateRequest {
    fn into_agent_view(self) -> Result<AgentView, RouterError> {
        if self.agent_id == 0 {
            return Err(RouterError::BadRequest(
                "agent_id must be greater than zero".to_owned(),
            ));
        }
        let readable_brains = if self.readable_brains.is_empty() {
            BTreeSet::from([BrainId(1)])
        } else {
            self.readable_brains.into_iter().map(BrainId).collect()
        };
        let readable_scopes = parse_scope_set(self.readable_scopes)?;
        let writable_scopes = parse_scope_set(self.writable_scopes)?;
        let allowed_modes = parse_or_default(
            self.allowed_modes,
            BTreeSet::from([RetrievalMode::Fast, RetrievalMode::Balanced]),
            "retrieval mode",
        )?;
        let allowed_memory_types = parse_or_default(
            self.allowed_memory_types,
            BTreeSet::from([
                MemoryType::Decision,
                MemoryType::Observation,
                MemoryType::WorkflowResult,
            ]),
            "memory type",
        )?;
        let private_scope = self
            .private_scope
            .as_deref()
            .map(validate_scope_label)
            .transpose()?
            .map(|scope| scope_id(&scope));
        Ok(AgentView {
            agent_id: AgentId(self.agent_id),
            label: self.label,
            readable_brains,
            readable_scopes,
            writable_scopes,
            allowed_modes,
            allowed_memory_types,
            max_context_budget_tokens: self.max_context_budget_tokens.unwrap_or(4_000),
            default_context_budget_tokens: self.default_context_budget_tokens.unwrap_or(1_000),
            max_candidate_limit: self.max_candidate_limit.unwrap_or(100),
            default_candidate_limit: self.default_candidate_limit.unwrap_or(20),
            min_required_confidence_q16: self.min_required_confidence_q16.unwrap_or(Q16_ZERO),
            max_ttl_seconds: self.max_ttl_seconds.or(Some(2_592_000)),
            allow_remember: self.allow_remember.unwrap_or(true),
            allow_verify_fact: self.allow_verify_fact.unwrap_or(true),
            allow_audit_mode: self.allow_audit_mode.unwrap_or(false),
            require_citations_by_default: self.require_citations_by_default.unwrap_or(false),
            private_scope,
        })
    }
}

fn parse_scope_set(scopes: Vec<String>) -> Result<BTreeSet<ScopeId>, RouterError> {
    scopes
        .into_iter()
        .map(|scope| validate_scope_label(&scope).map(|scope| scope_id(&scope)))
        .collect()
}

fn parse_or_default<T>(
    values: Vec<String>,
    default: BTreeSet<T>,
    label: &'static str,
) -> Result<BTreeSet<T>, RouterError>
where
    T: FromStr + Ord,
{
    if values.is_empty() {
        return Ok(default);
    }
    values
        .into_iter()
        .map(|value| {
            value
                .parse::<T>()
                .map_err(|_| RouterError::BadRequest(format!("invalid {label}: {}", value.trim())))
        })
        .collect()
}

fn validate_scope_label(scope: &str) -> Result<String, RouterError> {
    let scope = scope.trim();
    if scope.is_empty() {
        return Err(RouterError::BadRequest(
            "scope must not be empty".to_owned(),
        ));
    }
    if scope.chars().any(char::is_control) {
        return Err(RouterError::BadRequest(
            "scope must not contain control characters".to_owned(),
        ));
    }
    Ok(scope.to_owned())
}

fn agent_view_response(view: &AgentView) -> AgentViewResponse {
    AgentViewResponse {
        schema_version: "cortexdb.agent_view.v1",
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

fn mode_label(mode: &RetrievalMode) -> &'static str {
    match mode {
        RetrievalMode::Fast => "fast",
        RetrievalMode::Balanced => "balanced",
        RetrievalMode::Hybrid => "hybrid",
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
