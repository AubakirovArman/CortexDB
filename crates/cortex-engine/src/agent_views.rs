use std::collections::BTreeSet;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use cortex_aql::{AgentId, AgentView, BrainId, MemoryType, RetrievalMode, ScopeId};

use crate::database::Database;
use crate::error::{EngineError, EngineResult};

const HEADER: &str = "cortexdb.agent_view.v1";

impl Database {
    pub fn save_agent_view(&self, view: &AgentView) -> EngineResult<()> {
        let dir = agent_views_dir(&self.root_path);
        fs::create_dir_all(&dir)?;
        write_atomic(
            &agent_view_path(&dir, view.agent_id),
            encode_view(view).as_bytes(),
        )
    }

    pub fn load_agent_view(&self, agent_id: AgentId) -> EngineResult<Option<AgentView>> {
        let path = agent_view_path(&agent_views_dir(&self.root_path), agent_id);
        match fs::read_to_string(path) {
            Ok(text) => decode_view(&text).map(Some),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error.into()),
        }
    }

    pub fn list_agent_views(&self) -> EngineResult<Vec<AgentView>> {
        let dir = agent_views_dir(&self.root_path);
        let entries = match fs::read_dir(dir) {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(error) => return Err(error.into()),
        };
        let mut views = Vec::new();
        for entry in entries {
            let path = entry?.path();
            if path.extension().and_then(|value| value.to_str()) == Some("view") {
                views.push(decode_view(&fs::read_to_string(path)?)?);
            }
        }
        views.sort_by_key(|view| view.agent_id);
        Ok(views)
    }
}

fn agent_views_dir(root: &Path) -> PathBuf {
    root.join("agent_views")
}

fn agent_view_path(dir: &Path, agent_id: AgentId) -> PathBuf {
    dir.join(format!("{}.view", agent_id.0))
}

fn encode_view(view: &AgentView) -> String {
    [
        HEADER.to_owned(),
        format!("agent_id={}", view.agent_id.0),
        format!(
            "label_hex={}",
            encode_optional_string(view.label.as_deref())
        ),
        format!("readable_brains={}", join_brain_ids(&view.readable_brains)),
        format!("readable_scopes={}", join_scope_ids(&view.readable_scopes)),
        format!("writable_scopes={}", join_scope_ids(&view.writable_scopes)),
        format!("allowed_modes={}", join_modes(&view.allowed_modes)),
        format!(
            "allowed_memory_types={}",
            join_memory_types(&view.allowed_memory_types)
        ),
        format!(
            "max_context_budget_tokens={}",
            view.max_context_budget_tokens
        ),
        format!(
            "default_context_budget_tokens={}",
            view.default_context_budget_tokens
        ),
        format!("max_candidate_limit={}", view.max_candidate_limit),
        format!("default_candidate_limit={}", view.default_candidate_limit),
        format!(
            "min_required_confidence_q16={}",
            view.min_required_confidence_q16
        ),
        format!("max_ttl_seconds={}", option_u64(view.max_ttl_seconds)),
        format!("allow_remember={}", view.allow_remember),
        format!("allow_verify_fact={}", view.allow_verify_fact),
        format!("allow_audit_mode={}", view.allow_audit_mode),
        format!(
            "require_citations_by_default={}",
            view.require_citations_by_default
        ),
        format!(
            "private_scope={}",
            view.private_scope
                .map(|scope| scope.0.to_string())
                .unwrap_or_else(|| "none".to_owned())
        ),
    ]
    .join("\n")
}

fn decode_view(text: &str) -> EngineResult<AgentView> {
    let mut lines = text.lines();
    if lines.next() != Some(HEADER) {
        return Err(invalid_view("invalid agent view header"));
    }
    let fields = lines
        .filter_map(|line| line.split_once('='))
        .collect::<std::collections::BTreeMap<_, _>>();
    Ok(AgentView {
        agent_id: AgentId(
            required(&fields, "agent_id")?
                .parse()
                .map_err(invalid_parse)?,
        ),
        label: decode_optional_string(required(&fields, "label_hex")?)?,
        readable_brains: parse_id_set(required(&fields, "readable_brains")?, BrainId)?,
        readable_scopes: parse_id_set(required(&fields, "readable_scopes")?, ScopeId)?,
        writable_scopes: parse_id_set(required(&fields, "writable_scopes")?, ScopeId)?,
        allowed_modes: parse_enum_set(required(&fields, "allowed_modes")?)?,
        allowed_memory_types: parse_enum_set(required(&fields, "allowed_memory_types")?)?,
        max_context_budget_tokens: parse_field(&fields, "max_context_budget_tokens")?,
        default_context_budget_tokens: parse_field(&fields, "default_context_budget_tokens")?,
        max_candidate_limit: parse_field(&fields, "max_candidate_limit")?,
        default_candidate_limit: parse_field(&fields, "default_candidate_limit")?,
        min_required_confidence_q16: parse_field(&fields, "min_required_confidence_q16")?,
        max_ttl_seconds: parse_option_u64(required(&fields, "max_ttl_seconds")?)?,
        allow_remember: parse_field(&fields, "allow_remember")?,
        allow_verify_fact: parse_field(&fields, "allow_verify_fact")?,
        allow_audit_mode: parse_field(&fields, "allow_audit_mode")?,
        require_citations_by_default: parse_field(&fields, "require_citations_by_default")?,
        private_scope: parse_option_u64(required(&fields, "private_scope")?)?.map(ScopeId),
    })
}

fn write_atomic(path: &Path, bytes: &[u8]) -> EngineResult<()> {
    let tmp = path.with_extension("view.tmp");
    {
        let mut file = File::create(&tmp)?;
        file.write_all(bytes)?;
        file.sync_all()?;
    }
    fs::rename(&tmp, path)?;
    if let Some(parent) = path.parent() {
        File::open(parent)?.sync_all()?;
    }
    Ok(())
}

fn required<'a>(
    fields: &'a std::collections::BTreeMap<&str, &str>,
    key: &str,
) -> EngineResult<&'a str> {
    fields
        .get(key)
        .copied()
        .ok_or_else(|| invalid_view(format!("missing agent view field: {key}")))
}

fn parse_field<T: FromStr>(
    fields: &std::collections::BTreeMap<&str, &str>,
    key: &str,
) -> EngineResult<T>
where
    T::Err: std::fmt::Display,
{
    required(fields, key)?.parse().map_err(invalid_parse)
}

fn parse_id_set<T>(value: &str, wrap: fn(u64) -> T) -> EngineResult<BTreeSet<T>>
where
    T: Ord,
{
    parse_csv(value)
        .map(|raw| raw.parse().map(wrap).map_err(invalid_parse))
        .collect()
}

fn parse_enum_set<T: FromStr + Ord>(value: &str) -> EngineResult<BTreeSet<T>> {
    parse_csv(value)
        .map(|raw| raw.parse().map_err(|_| invalid_view("invalid enum value")))
        .collect()
}

fn parse_csv(value: &str) -> impl Iterator<Item = &str> {
    value.split(',').filter(|part| !part.is_empty())
}

fn parse_option_u64(value: &str) -> EngineResult<Option<u64>> {
    if value == "none" {
        Ok(None)
    } else {
        value.parse().map(Some).map_err(invalid_parse)
    }
}

fn join_brain_ids(values: &BTreeSet<BrainId>) -> String {
    values
        .iter()
        .map(|value| value.0.to_string())
        .collect::<Vec<_>>()
        .join(",")
}

fn join_scope_ids(values: &BTreeSet<ScopeId>) -> String {
    values
        .iter()
        .map(|value| value.0.to_string())
        .collect::<Vec<_>>()
        .join(",")
}

fn join_modes(values: &BTreeSet<RetrievalMode>) -> String {
    values
        .iter()
        .map(|mode| match mode {
            RetrievalMode::Fast => "fast",
            RetrievalMode::Balanced => "balanced",
            RetrievalMode::Semantic => "semantic",
            RetrievalMode::Audit => "audit",
        })
        .collect::<Vec<_>>()
        .join(",")
}

fn join_memory_types(values: &BTreeSet<MemoryType>) -> String {
    values
        .iter()
        .map(|memory_type| match memory_type {
            MemoryType::Decision => "decision",
            MemoryType::Preference => "preference",
            MemoryType::WorkflowResult => "workflow_result",
            MemoryType::ErrorLog => "error_log",
            MemoryType::Observation => "observation",
        })
        .collect::<Vec<_>>()
        .join(",")
}

fn option_u64(value: Option<u64>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "none".to_owned())
}

fn encode_optional_string(value: Option<&str>) -> String {
    value.map(hex_encode).unwrap_or_else(|| "none".to_owned())
}

fn decode_optional_string(value: &str) -> EngineResult<Option<String>> {
    if value == "none" {
        return Ok(None);
    }
    let bytes = hex_decode(value)?;
    String::from_utf8(bytes).map(Some).map_err(invalid_parse)
}

fn hex_encode(value: &str) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(value.len() * 2);
    for byte in value.as_bytes() {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

fn hex_decode(value: &str) -> EngineResult<Vec<u8>> {
    if !value.len().is_multiple_of(2) {
        return Err(invalid_view("invalid hex length"));
    }
    value
        .as_bytes()
        .chunks_exact(2)
        .map(|chunk| Ok((hex_value(chunk[0])? << 4) | hex_value(chunk[1])?))
        .collect()
}

fn hex_value(value: u8) -> EngineResult<u8> {
    match value {
        b'0'..=b'9' => Ok(value - b'0'),
        b'a'..=b'f' => Ok(value - b'a' + 10),
        b'A'..=b'F' => Ok(value - b'A' + 10),
        _ => Err(invalid_view("invalid hex digit")),
    }
}

fn invalid_parse(error: impl std::fmt::Display) -> EngineError {
    invalid_view(format!("invalid agent view field: {error}"))
}

fn invalid_view(message: impl Into<String>) -> EngineError {
    EngineError::StorageInvariant(format!("agent view store: {}", message.into()))
}
