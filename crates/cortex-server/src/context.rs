use cortex_aql::{AgentId, AgentView, BrainId, MemoryType, RetrievalMode, Q16_ZERO};
use cortex_engine::{scope_id, ContextPack, ContextPackOptions, Database};

use crate::responses::{
    ContextPackAnomalyResponse, ContextPackCellResponse, ContextPackResponse, ExplainResponse,
    RouterError, SourceRefResponse,
};

use crate::router::query_param_decoded;

pub fn handle_context_shared(
    db: &Database,
    query: &str,
    body: &[u8],
) -> Result<String, RouterError> {
    let scope = query_param_decoded(query, "scope").map_err(RouterError::BadRequest)?;
    let aql = String::from_utf8_lossy(body);
    let pack =
        db.context_pack_from_aql(&aql, &view_for_scope(&scope), ContextPackOptions::default())?;

    let response = map_context_pack(&pack);
    Ok(serde_json::to_string(&response)?)
}

pub(crate) fn view_for_scope(scope: &str) -> AgentView {
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

fn map_context_pack(pack: &ContextPack) -> ContextPackResponse {
    let cells = pack
        .cells
        .iter()
        .map(|cell| {
            let metadata = cortex_engine::query::CellMetadata::from_payload(&cell.payload);
            let source_ref = metadata.source_ref.as_ref().map(|sr| SourceRefResponse {
                source_id: sr.source_id.clone(),
                document_id: sr.document_id.clone(),
                page: sr.page,
                cell_range: sr.cell_range.clone(),
                json_path: sr.json_path.clone(),
                confidence_q16: sr.confidence_q16,
            });

            let explain = cell.explain.as_ref().map(|exp| ExplainResponse {
                score: exp.score,
                matched_terms: exp.matched_terms.clone(),
                why_selected: exp.why_selected.clone(),
                base_bm25: exp.base_bm25,
                source_trust_bonus: exp.source_trust_bonus,
                redundancy_penalty: exp.redundancy_penalty,
            });

            ContextPackCellResponse {
                cell_id: cell.cell_id.0,
                estimated_tokens: cell.estimated_tokens,
                citation: cell.citation.clone(),
                payload_text: String::from_utf8_lossy(&cell.payload).into_owned(),
                explain,
                source_ref,
            }
        })
        .collect();

    let anomalies = pack
        .anomalies
        .iter()
        .map(|anom| ContextPackAnomalyResponse {
            cell_id: anom.cell_id.map(|cid| cid.0),
            code: anom.code.as_str().to_owned(),
            message: anom.message.clone(),
        })
        .collect();

    ContextPackResponse {
        schema_version: "context_pack.v1",
        token_budget_tokens: pack.token_budget_tokens,
        estimated_tokens: pack.estimated_tokens,
        truncated: pack.truncated,
        citations_required: pack.citations_required,
        cells,
        anomalies,
    }
}
