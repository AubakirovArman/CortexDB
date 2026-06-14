use crate::cli_json_types::{
    ContextAccessDecisionResponse, ContextCellExplainResponse, ContextPackAnomalyResponse,
    ContextPackCellResponse, ContextPackExplainResponse, ContextPackResponse,
    ContextPackScoreComponentResponse, ContextSpanProvenanceResponse, SourceRefResponse,
};

pub(super) fn context_cell_explain_response(
    explain: &cortex_engine::ContextCellExplain,
) -> ContextCellExplainResponse {
    ContextCellExplainResponse {
        schema_version: "context_cell_explain.v1",
        cell_id: explain.cell_id.0,
        outcome: explain.outcome.as_str().to_owned(),
        first_excluding_stage: explain.first_excluding_stage.clone(),
        why_selected: explain.why_selected.clone(),
        why_excluded: explain.why_excluded.clone(),
        score: explain.score,
        matched_terms: explain.matched_terms.clone(),
        score_components: explain
            .score_components
            .iter()
            .map(score_component_response)
            .collect(),
        access_decision: explain.access_decision.as_ref().map(|decision| {
            ContextAccessDecisionResponse {
                decision: decision.decision.as_str().to_owned(),
                policy: decision.policy.clone(),
                reason: decision.reason.clone(),
                scope: decision.scope.clone(),
                scope_id: decision.scope_id,
                agent_id: decision.agent_id,
            }
        }),
    }
}

pub(super) fn context_pack_response(pack: &cortex_engine::ContextPack) -> ContextPackResponse {
    ContextPackResponse {
        schema_version: "context_pack.v1",
        token_budget_tokens: pack.token_budget_tokens,
        estimated_tokens: pack.estimated_tokens,
        truncated: pack.truncated,
        citations_required: pack.citations_required,
        answerability_q16: pack.answerability_q16,
        conflict_visibility_q16: pack.conflict_visibility_q16,
        visible_conflict_count: pack.visible_conflict_count,
        cells: pack.cells.iter().map(context_cell_json).collect(),
        anomalies: pack
            .anomalies
            .iter()
            .map(|anomaly| ContextPackAnomalyResponse {
                cell_id: anomaly.cell_id.map(|id| id.0),
                code: anomaly.code.as_str().to_owned(),
                message: anomaly.message.clone(),
                why_excluded: anomaly.why_excluded.clone(),
            })
            .collect(),
    }
}

fn context_cell_json(cell: &cortex_engine::ContextPackCell) -> ContextPackCellResponse {
    let explain = cell.explain.as_ref().map(|exp| ContextPackExplainResponse {
        score: exp.score,
        matched_terms: exp.matched_terms.clone(),
        why_selected: exp.why_selected.clone(),
        score_components: exp
            .score_components
            .iter()
            .map(score_component_response)
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
    let source_ref = cell.metadata.source_ref.as_ref().map(source_ref_response);
    let provenance = cell
        .provenance
        .as_ref()
        .map(|provenance| ContextSpanProvenanceResponse {
            source_cell_id: provenance.source_cell_id.0,
            source_byte_start: provenance.source_byte_start,
            source_byte_end: provenance.source_byte_end,
            source_line_start: provenance.source_line_start,
            source_line_end: provenance.source_line_end,
            source_ref: provenance.source_ref.as_ref().map(source_ref_response),
        });

    ContextPackCellResponse {
        cell_id: cell.cell_id.0,
        estimated_tokens: cell.estimated_tokens,
        citation: cell.citation.clone(),
        payload_text: String::from_utf8_lossy(&cell.payload).into_owned(),
        explain,
        source_ref,
        provenance,
    }
}

fn score_component_response(
    component: &cortex_engine::ContextScoreComponent,
) -> ContextPackScoreComponentResponse {
    ContextPackScoreComponentResponse {
        name: component.name.clone(),
        value: component.value,
        contribution: component.contribution,
        reason: component.reason.clone(),
    }
}

fn source_ref_response(sr: &cortex_engine::SourceRef) -> SourceRefResponse {
    SourceRefResponse {
        source_id: sr.source_id.clone(),
        source_url: sr.source_url.clone(),
        document_id: sr.document_id.clone(),
        page: sr.page,
        cell_range: sr.cell_range.clone(),
        json_path: sr.json_path.clone(),
        confidence_q16: sr.confidence_q16,
    }
}
