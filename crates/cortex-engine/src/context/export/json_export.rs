use serde_json::json;

use crate::context::ContextPack;
use crate::query::metadata::SourceRef;

pub(super) fn to_json(pack: &ContextPack) -> String {
    let cells = pack
        .cells
        .iter()
        .map(|cell| {
            json!({
                "cell_id": cell.cell_id.0,
                "estimated_tokens": cell.estimated_tokens,
                "citation": cell.citation.as_ref(),
                "payload_text": String::from_utf8_lossy(&cell.payload),
                "source_ref": cell.metadata.source_ref.as_ref().map(source_ref_json),
                "provenance": cell.provenance.as_ref().map(|provenance| json!({
                    "source_cell_id": provenance.source_cell_id.0,
                    "source_byte_start": provenance.source_byte_start,
                    "source_byte_end": provenance.source_byte_end,
                    "source_line_start": provenance.source_line_start,
                    "source_line_end": provenance.source_line_end,
                    "source_ref": provenance.source_ref.as_ref().map(source_ref_json),
                })),
                "access_decision": cell.access_decision.as_ref().map(|decision| json!({
                    "cell_id": decision.cell_id.0,
                    "decision": decision.decision.as_str(),
                    "policy": &decision.policy,
                    "reason": &decision.reason,
                    "scope": &decision.scope,
                    "scope_id": decision.scope_id,
                    "agent_id": decision.agent_id,
                })),
                "explain": cell.explain.as_ref().map(|explain| json!({
                    "score": explain.score,
                    "matched_terms": &explain.matched_terms,
                    "why_selected": &explain.why_selected,
                    "score_components": explain.score_components.iter().map(|component| json!({
                        "name": &component.name,
                        "value": component.value,
                        "contribution": component.contribution,
                        "reason": &component.reason,
                    })).collect::<Vec<_>>(),
                    "base_bm25": explain.base_bm25,
                    "source_trust_q16": explain.source_trust_q16,
                    "source_trust_category": explain.source_trust_category.as_str(),
                    "source_trust_bonus": explain.source_trust_bonus,
                    "source_freshness_q16": explain.source_freshness_q16,
                    "source_freshness_category": explain.source_freshness_category.as_str(),
                    "source_freshness_bonus": explain.source_freshness_bonus,
                    "redundancy_penalty": explain.redundancy_penalty,
                })),
            })
        })
        .collect::<Vec<_>>();
    let anomalies = pack
        .anomalies
        .iter()
        .map(|anomaly| {
            json!({
                "cell_id": anomaly.cell_id.map(|cell_id| cell_id.0),
                "code": anomaly.code.as_str(),
                "message": &anomaly.message,
                "why_excluded": anomaly.why_excluded.as_ref(),
            })
        })
        .collect::<Vec<_>>();
    json!({
        "schema_version": "context_pack.v1",
        "token_budget_tokens": pack.token_budget_tokens,
        "estimated_tokens": pack.estimated_tokens,
        "truncated": pack.truncated,
        "citations_required": pack.citations_required,
        "answerability_q16": pack.answerability_q16,
        "conflict_visibility_q16": pack.conflict_visibility_q16,
        "visible_conflict_count": pack.visible_conflict_count,
        "cells": cells,
        "anomalies": anomalies,
    })
    .to_string()
}

fn source_ref_json(source_ref: &SourceRef) -> serde_json::Value {
    json!({
        "source_id": &source_ref.source_id,
        "source_url": &source_ref.source_url,
        "document_id": &source_ref.document_id,
        "page": source_ref.page,
        "row": source_ref.row,
        "cell_range": &source_ref.cell_range,
        "json_path": &source_ref.json_path,
        "confidence_q16": source_ref.confidence_q16,
    })
}
