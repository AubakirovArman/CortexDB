use cortex_core::CellId;
use cortex_engine::{CellMetadata, ContextPack, ContextPackCell};

pub(super) use crate::responses::map_answer_grounding_report;

use super::LlmInferenceRequest;

pub(super) fn request_has_api_key(request: &LlmInferenceRequest) -> bool {
    request
        .api_key
        .as_deref()
        .is_some_and(|value| !value.trim().is_empty())
}

pub(super) fn citations(request: &LlmInferenceRequest) -> Vec<String> {
    request
        .context_pack
        .cells
        .iter()
        .filter_map(|cell| cell.citation.clone().or_else(|| cell.source_ref.clone()))
        .collect()
}

pub(super) fn citation_count(request: &LlmInferenceRequest) -> usize {
    request
        .context_pack
        .cells
        .iter()
        .filter(|cell| cell.citation.is_some() || cell.source_ref.is_some())
        .count()
}

pub(super) fn summarize_from_context(text: &str) -> String {
    let snippet = text.chars().take(180).collect::<String>();
    format!("Test-double answer from explicit ContextPack only: {snippet}")
}

pub(super) fn grounding_context_pack(request: &LlmInferenceRequest) -> ContextPack {
    ContextPack {
        cells: request
            .context_pack
            .cells
            .iter()
            .map(|cell| {
                let payload = cell
                    .text
                    .as_deref()
                    .or(cell.payload_text.as_deref())
                    .unwrap_or("")
                    .as_bytes()
                    .to_vec();
                ContextPackCell {
                    cell_id: CellId(cell.cell_id),
                    metadata: CellMetadata::from_payload(&payload),
                    payload,
                    estimated_tokens: 0,
                    citation: cell.citation.clone().or_else(|| cell.source_ref.clone()),
                    provenance: None,
                    explain: None,
                    access_decision: None,
                }
            })
            .collect(),
        token_budget_tokens: 0,
        estimated_tokens: 0,
        truncated: false,
        citations_required: true,
        answerability_q16: u16::MAX,
        conflict_visibility_q16: 0,
        visible_conflict_count: 0,
        anomalies: Vec::new(),
        grounding_report: None,
    }
}
