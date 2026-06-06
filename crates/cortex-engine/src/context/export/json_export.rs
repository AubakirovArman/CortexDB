use serde_json::json;

use crate::context::ContextPack;
use crate::query::metadata::SourceRef;
use crate::query::CellMetadata;

pub(super) fn to_json(pack: &ContextPack) -> String {
    let cells = pack
        .cells
        .iter()
        .map(|cell| {
            let metadata = CellMetadata::from_payload(&cell.payload);
            json!({
                "cell_id": cell.cell_id.0,
                "estimated_tokens": cell.estimated_tokens,
                "citation": cell.citation.as_ref(),
                "payload_text": String::from_utf8_lossy(&cell.payload),
                "source_ref": metadata.source_ref.as_ref().map(source_ref_json),
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
