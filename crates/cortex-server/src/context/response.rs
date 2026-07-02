use cortex_engine::ContextPack;

use crate::auth::AuthRouteContext;
use crate::responses::{
    map_answer_grounding_report, ContextAccessDecisionResponse, ContextPackAnomalyResponse,
    ContextPackCellResponse, ContextPackResponse, ContextSpanProvenanceResponse, ExplainResponse,
    ScoreComponentResponse, SourceRefResponse,
};

pub(crate) fn map_context_pack(
    pack: &ContextPack,
    auth_context: Option<&AuthRouteContext>,
    accountability_receipt: Option<serde_json::Value>,
) -> ContextPackResponse {
    let cells = pack
        .cells
        .iter()
        .map(|cell| {
            let source_ref = cell.metadata.source_ref.as_ref().map(map_source_ref);
            let provenance =
                cell.provenance
                    .as_ref()
                    .map(|provenance| ContextSpanProvenanceResponse {
                        source_cell_id: provenance.source_cell_id.0,
                        source_byte_start: provenance.source_byte_start,
                        source_byte_end: provenance.source_byte_end,
                        source_line_start: provenance.source_line_start,
                        source_line_end: provenance.source_line_end,
                        source_ref: provenance.source_ref.as_ref().map(map_source_ref),
                    });

            let explain = cell.explain.as_ref().map(|exp| ExplainResponse {
                score: exp.score,
                matched_terms: exp.matched_terms.clone(),
                why_selected: exp.why_selected.clone(),
                score_components: exp
                    .score_components
                    .iter()
                    .map(|component| ScoreComponentResponse {
                        name: component.name.clone(),
                        value: component.value,
                        contribution: component.contribution,
                        reason: component.reason.clone(),
                    })
                    .collect(),
                base_bm25: exp.base_bm25,
                source_trust_q16: exp.source_trust_q16,
                source_trust_category: exp.source_trust_category.as_str().to_owned(),
                source_trust_bonus: exp.source_trust_bonus,
                source_freshness_q16: exp.source_freshness_q16,
                source_freshness_category: exp.source_freshness_category.as_str().to_owned(),
                source_freshness_bonus: exp.source_freshness_bonus,
                redundancy_penalty: exp.redundancy_penalty,
            });

            ContextPackCellResponse {
                cell_id: cell.cell_id.0,
                estimated_tokens: cell.estimated_tokens,
                citation: cell.citation.clone(),
                payload_text: String::from_utf8_lossy(&cell.payload).into_owned(),
                explain,
                source_ref,
                provenance,
                access_decision: cell.access_decision.as_ref().map(|decision| {
                    ContextAccessDecisionResponse {
                        cell_id: decision.cell_id.0,
                        decision: decision.decision.as_str().to_owned(),
                        policy: decision.policy.clone(),
                        policy_version: decision.policy_version.clone(),
                        reason: decision.reason.clone(),
                        scope: decision.scope.clone(),
                        scope_id: decision.scope_id,
                        agent_id: decision.agent_id,
                        agent_view_digest: decision.agent_view_digest.clone(),
                        principal_id: auth_context.and_then(|ctx| ctx.principal_id.clone()),
                        auth_role: auth_context
                            .and_then(|ctx| ctx.role.map(|role| role.as_str().to_owned())),
                    }
                }),
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
            why_excluded: anom.why_excluded.clone(),
        })
        .collect();

    ContextPackResponse {
        schema_version: "context_pack.v1",
        token_budget_tokens: pack.token_budget_tokens,
        estimated_tokens: pack.estimated_tokens,
        truncated: pack.truncated,
        citations_required: pack.citations_required,
        answerability_q16: pack.answerability_q16,
        conflict_visibility_q16: pack.conflict_visibility_q16,
        visible_conflict_count: pack.visible_conflict_count,
        cells,
        anomalies,
        grounding_report: pack
            .grounding_report
            .as_ref()
            .map(map_answer_grounding_report),
        accountability_receipt,
    }
}

fn map_source_ref(sr: &cortex_engine::SourceRef) -> SourceRefResponse {
    SourceRefResponse {
        source_id: sr.source_id.clone(),
        source_url: sr.source_url.clone(),
        document_id: sr.document_id.clone(),
        page: sr.page,
        row: sr.row,
        cell_range: sr.cell_range.clone(),
        json_path: sr.json_path.clone(),
        confidence_q16: sr.confidence_q16,
    }
}
