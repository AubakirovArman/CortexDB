use std::path::Path;

use cortex_aql::{AgentId, AgentView, BrainId, MemoryType, RetrievalMode, Q16_ZERO};
use cortex_engine::{scope_id, ContextPack, ContextPackOptions, Database};

pub fn handle_context(root: &Path, query: &str, body: &[u8]) -> Result<String, String> {
    let scope = query_param(query, "scope")?;
    let db = Database::open(root).map_err(|error| error.to_string())?;
    let aql = String::from_utf8_lossy(body);
    let pack = db
        .context_pack_from_aql(&aql, &view_for_scope(scope), ContextPackOptions::default())
        .map_err(|error| error.to_string())?;
    Ok(context_pack_json(&pack))
}

fn query_param<'a>(query: &'a str, key: &str) -> Result<&'a str, String> {
    let prefix = format!("{key}=");
    query
        .split('&')
        .find_map(|pair| pair.strip_prefix(&prefix))
        .ok_or_else(|| format!("missing {key}"))
}

fn view_for_scope(scope: &str) -> AgentView {
    AgentView {
        agent_id: AgentId(1),
        label: Some("local-http".to_owned()),
        readable_brains: std::collections::BTreeSet::from([BrainId(1)]),
        readable_scopes: std::collections::BTreeSet::from([scope_id(scope)]),
        writable_scopes: std::collections::BTreeSet::new(),
        allowed_modes: std::collections::BTreeSet::from([RetrievalMode::Balanced]),
        allowed_memory_types: std::collections::BTreeSet::from([MemoryType::Decision]),
        max_context_budget_tokens: 4_000,
        default_context_budget_tokens: 1_000,
        max_candidate_limit: 100,
        default_candidate_limit: 20,
        min_required_confidence_q16: Q16_ZERO,
        max_ttl_seconds: Some(3_600),
        allow_remember: false,
        allow_verify_fact: false,
        allow_audit_mode: false,
        require_citations_by_default: false,
        private_scope: None,
    }
}

fn context_pack_json(pack: &ContextPack) -> String {
    let cells = pack
        .cells
        .iter()
        .map(context_cell_json)
        .collect::<Vec<_>>()
        .join(",");
    format!(
        r#"{{"token_budget":{},"estimated_tokens":{},"truncated":{},"citations_required":{},"cells":[{}],"anomalies":[{}]}}"#,
        pack.token_budget_tokens,
        pack.estimated_tokens,
        pack.truncated,
        pack.citations_required,
        cells,
        anomaly_json(pack)
    )
}

fn context_cell_json(cell: &cortex_engine::ContextPackCell) -> String {
    format!(
        r#"{{"cell_id":{},"estimated_tokens":{},"citation":{},"payload":"{}"}}"#,
        cell.cell_id.0,
        cell.estimated_tokens,
        json_optional_string(cell.citation.as_deref()),
        escape_json(&String::from_utf8_lossy(&cell.payload))
    )
}

fn anomaly_json(pack: &ContextPack) -> String {
    pack.anomalies
        .iter()
        .map(|anomaly| {
            let cell_id = anomaly
                .cell_id
                .map(|cell_id| cell_id.0.to_string())
                .unwrap_or_else(|| "null".to_owned());
            format!(
                r#"{{"cell_id":{},"code":"{}","message":"{}"}}"#,
                cell_id,
                escape_json(anomaly.code),
                escape_json(&anomaly.message)
            )
        })
        .collect::<Vec<_>>()
        .join(",")
}

fn json_optional_string(value: Option<&str>) -> String {
    value
        .map(|value| format!(r#""{}""#, escape_json(value)))
        .unwrap_or_else(|| "null".to_owned())
}

fn escape_json(value: &str) -> String {
    value
        .chars()
        .flat_map(|character| match character {
            '"' => "\\\"".chars().collect::<Vec<_>>(),
            '\\' => "\\\\".chars().collect(),
            '\n' => "\\n".chars().collect(),
            '\r' => "\\r".chars().collect(),
            '\t' => "\\t".chars().collect(),
            other => vec![other],
        })
        .collect()
}
