use std::collections::BTreeSet;
use std::str::FromStr;

use cortex_aql::{AgentId, AgentView, BrainId, MemoryType, RetrievalMode, ScopeId};
use cortex_engine::{scope_id, Database};

use super::super::{fmt_engine_error, open_database};

mod format;

use format::{format_agent_list, format_agent_response, format_scope_mutation};

pub(crate) struct AgentCreateInput {
    pub(crate) path: String,
    pub(crate) agent_id: u64,
    pub(crate) label: Option<String>,
    pub(crate) readable_scopes: Vec<String>,
    pub(crate) writable_scopes: Vec<String>,
    pub(crate) readable_brains: Vec<u64>,
    pub(crate) allowed_modes: Vec<String>,
    pub(crate) allowed_memory_types: Vec<String>,
    pub(crate) max_context_budget_tokens: u32,
    pub(crate) default_context_budget_tokens: u32,
    pub(crate) max_candidate_limit: u32,
    pub(crate) default_candidate_limit: u32,
    pub(crate) min_required_confidence_q16: u16,
    pub(crate) max_ttl_seconds: Option<u64>,
    pub(crate) private_scope: Option<String>,
    pub(crate) allow_remember: bool,
    pub(crate) allow_verify_fact: bool,
    pub(crate) allow_audit_mode: bool,
    pub(crate) require_citations_by_default: bool,
}

pub(crate) struct AgentScopeInput {
    pub(crate) path: String,
    pub(crate) agent_id: u64,
    pub(crate) scope: String,
    pub(crate) access: AgentScopeAccess,
}

#[derive(Clone, Copy)]
pub(crate) enum AgentScopeAccess {
    Read,
    Write,
    ReadWrite,
}

pub(crate) fn create_agent(json_output: bool, input: AgentCreateInput) -> Result<String, String> {
    let db = open_database(&input.path, false)?;
    let view = input.into_agent_view()?;
    db.save_agent_view(&view).map_err(fmt_engine_error)?;
    format_agent_response(json_output, &view)
}

pub(crate) fn list_agents(json_output: bool, path: String) -> Result<String, String> {
    let db = open_database(&path, false)?;
    let views = db.list_agent_views().map_err(fmt_engine_error)?;
    format_agent_list(json_output, &views)
}

pub(crate) fn show_agent(json_output: bool, path: String, agent_id: u64) -> Result<String, String> {
    let db = open_database(&path, false)?;
    let view = load_agent_view(&db, agent_id)?;
    format_agent_response(json_output, &view)
}

pub(crate) fn grant_agent_scope(
    json_output: bool,
    input: AgentScopeInput,
) -> Result<String, String> {
    mutate_agent_scope(json_output, input, true)
}

pub(crate) fn revoke_agent_scope(
    json_output: bool,
    input: AgentScopeInput,
) -> Result<String, String> {
    mutate_agent_scope(json_output, input, false)
}

impl AgentCreateInput {
    fn into_agent_view(self) -> Result<AgentView, String> {
        validate_agent_id(self.agent_id)?;
        if self.default_context_budget_tokens > self.max_context_budget_tokens {
            return Err("default context budget cannot exceed max context budget".to_owned());
        }
        if self.default_candidate_limit > self.max_candidate_limit {
            return Err("default candidate limit cannot exceed max candidate limit".to_owned());
        }
        let readable_brains = if self.readable_brains.is_empty() {
            BTreeSet::from([BrainId(1)])
        } else {
            self.readable_brains.into_iter().map(BrainId).collect()
        };
        Ok(AgentView {
            agent_id: AgentId(self.agent_id),
            label: self.label,
            readable_brains,
            readable_scopes: parse_scopes(self.readable_scopes)?,
            writable_scopes: parse_scopes(self.writable_scopes)?,
            allowed_modes: parse_or_default(
                self.allowed_modes,
                BTreeSet::from([RetrievalMode::Fast, RetrievalMode::Balanced]),
                "retrieval mode",
            )?,
            allowed_memory_types: parse_or_default(
                self.allowed_memory_types,
                BTreeSet::from([
                    MemoryType::Decision,
                    MemoryType::Observation,
                    MemoryType::WorkflowResult,
                ]),
                "memory type",
            )?,
            max_context_budget_tokens: self.max_context_budget_tokens,
            default_context_budget_tokens: self.default_context_budget_tokens,
            max_candidate_limit: self.max_candidate_limit,
            default_candidate_limit: self.default_candidate_limit,
            min_required_confidence_q16: self.min_required_confidence_q16,
            max_ttl_seconds: self.max_ttl_seconds.or(Some(2_592_000)),
            allow_remember: self.allow_remember,
            allow_verify_fact: self.allow_verify_fact,
            allow_audit_mode: self.allow_audit_mode,
            require_citations_by_default: self.require_citations_by_default,
            private_scope: self
                .private_scope
                .as_deref()
                .map(validate_scope_label)
                .transpose()?
                .map(|scope| scope_id(&scope)),
        })
    }
}

fn mutate_agent_scope(
    json_output: bool,
    input: AgentScopeInput,
    grant: bool,
) -> Result<String, String> {
    let db = open_database(&input.path, false)?;
    validate_agent_id(input.agent_id)?;
    let scope = validate_scope_label(&input.scope)?;
    let mut view = load_agent_view(&db, input.agent_id)?;
    let scope_id = scope_id(&scope);
    if grant {
        input.access.apply_insert(&mut view, scope_id);
    } else {
        input.access.apply_remove(&mut view, scope_id);
    }
    db.save_agent_view(&view).map_err(fmt_engine_error)?;
    format_scope_mutation(
        json_output,
        if grant { "grant_scope" } else { "revoke_scope" },
        input.agent_id,
        &scope,
        input.access.as_str(),
        view.readable_scopes.len(),
        view.writable_scopes.len(),
    )
}

impl AgentScopeAccess {
    fn as_str(self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::Write => "write",
            Self::ReadWrite => "read_write",
        }
    }

    fn apply_insert(self, view: &mut AgentView, scope_id: ScopeId) {
        if matches!(self, Self::Read | Self::ReadWrite) {
            view.readable_scopes.insert(scope_id);
        }
        if matches!(self, Self::Write | Self::ReadWrite) {
            view.writable_scopes.insert(scope_id);
        }
    }

    fn apply_remove(self, view: &mut AgentView, scope_id: ScopeId) {
        if matches!(self, Self::Read | Self::ReadWrite) {
            view.readable_scopes.remove(&scope_id);
        }
        if matches!(self, Self::Write | Self::ReadWrite) {
            view.writable_scopes.remove(&scope_id);
        }
    }
}

fn load_agent_view(db: &Database, agent_id: u64) -> Result<AgentView, String> {
    validate_agent_id(agent_id)?;
    db.load_agent_view(AgentId(agent_id))
        .map_err(fmt_engine_error)?
        .ok_or_else(|| format!("agent view not found: {agent_id}"))
}

fn parse_scopes(values: Vec<String>) -> Result<BTreeSet<ScopeId>, String> {
    values
        .into_iter()
        .map(|scope| validate_scope_label(&scope).map(|scope| scope_id(&scope)))
        .collect()
}

fn parse_or_default<T>(
    values: Vec<String>,
    default: BTreeSet<T>,
    label: &'static str,
) -> Result<BTreeSet<T>, String>
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
                .map_err(|_| format!("invalid {label}: {}", value.trim()))
        })
        .collect()
}

fn validate_agent_id(agent_id: u64) -> Result<(), String> {
    if agent_id == 0 {
        Err("agent_id must be greater than zero".to_owned())
    } else {
        Ok(())
    }
}

fn validate_scope_label(scope: &str) -> Result<String, String> {
    let scope = scope.trim();
    if scope.is_empty() {
        return Err("scope must not be empty".to_owned());
    }
    if scope.chars().any(char::is_control) {
        return Err("scope must not contain control characters".to_owned());
    }
    Ok(scope.to_owned())
}
