use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet};

use cortex_aql::AgentView;
use cortex_core::CellId;

use crate::database::{Database, RetrievedCell};
use crate::error::EngineResult;
use crate::query::CellMetadata;
use crate::search::tokenize;

const DEFAULT_REDUNDANCY_THRESHOLD_Q16: u16 = 32_768;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContextPackOptions {
    pub token_budget_tokens: u32,
    pub require_citations: bool,
    pub reduce_redundancy: bool,
    pub redundancy_threshold_q16: u16,
}

impl Default for ContextPackOptions {
    fn default() -> Self {
        Self {
            token_budget_tokens: 0,
            require_citations: false,
            reduce_redundancy: false,
            redundancy_threshold_q16: DEFAULT_REDUNDANCY_THRESHOLD_Q16,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContextPack {
    pub cells: Vec<ContextPackCell>,
    pub token_budget_tokens: u32,
    pub estimated_tokens: u32,
    pub truncated: bool,
    pub citations_required: bool,
    pub anomalies: Vec<ContextPackAnomaly>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContextPackCell {
    pub cell_id: CellId,
    pub payload: Vec<u8>,
    pub estimated_tokens: u32,
    pub citation: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContextPackAnomaly {
    pub cell_id: Option<CellId>,
    pub code: &'static str,
    pub message: String,
}

impl Database {
    pub fn context_pack_from_aql(
        &self,
        aql: &str,
        view: &AgentView,
        options: ContextPackOptions,
    ) -> EngineResult<ContextPack> {
        let budget = effective_budget(view, options.token_budget_tokens);
        let citations_required = options.require_citations || view.require_citations_by_default;
        let cells = order_by_feedback(self.retrieve_aql(aql, view)?, &self.feedback_scores());
        Ok(ContextPack::from_retrieved_with_options(
            cells,
            budget,
            citations_required,
            &options,
        ))
    }
}

impl ContextPack {
    pub fn from_retrieved(
        cells: Vec<RetrievedCell>,
        token_budget_tokens: u32,
        citations_required: bool,
    ) -> Self {
        Self::from_retrieved_with_options(
            cells,
            token_budget_tokens,
            citations_required,
            &ContextPackOptions::default(),
        )
    }

    pub fn from_retrieved_with_options(
        cells: Vec<RetrievedCell>,
        token_budget_tokens: u32,
        citations_required: bool,
        options: &ContextPackOptions,
    ) -> Self {
        let mut pack_cells = Vec::new();
        let mut estimated_tokens = 0u32;
        let mut truncated = false;
        let mut anomalies = Vec::new();

        for cell in cells {
            let cell_tokens = estimate_tokens(&cell.payload);
            let would_exceed = !pack_cells.is_empty()
                && estimated_tokens.saturating_add(cell_tokens) > token_budget_tokens;
            if would_exceed {
                truncated = true;
                break;
            }
            if pack_cells.is_empty() && cell_tokens > token_budget_tokens {
                truncated = true;
            }

            if options.reduce_redundancy
                && is_redundant(
                    &cell.payload,
                    &pack_cells,
                    effective_redundancy_threshold(options.redundancy_threshold_q16),
                )
            {
                anomalies.push(ContextPackAnomaly {
                    cell_id: Some(cell.cell_id),
                    code: "redundant_cell",
                    message: "selected cell is redundant with an earlier packed cell".to_owned(),
                });
                continue;
            }

            let citation = extract_citation(&cell.payload);
            if citations_required && citation.is_none() {
                anomalies.push(ContextPackAnomaly {
                    cell_id: Some(cell.cell_id),
                    code: "missing_citation",
                    message: "selected cell does not include source= or citation=".to_owned(),
                });
            }
            estimated_tokens = estimated_tokens.saturating_add(cell_tokens);
            pack_cells.push(ContextPackCell {
                cell_id: cell.cell_id,
                payload: cell.payload,
                estimated_tokens: cell_tokens,
                citation,
            });
        }

        Self {
            cells: pack_cells,
            token_budget_tokens,
            estimated_tokens,
            truncated,
            citations_required,
            anomalies,
        }
    }
}

fn order_by_feedback(
    cells: Vec<RetrievedCell>,
    feedback_scores: &BTreeMap<CellId, i32>,
) -> Vec<RetrievedCell> {
    let mut indexed = cells.into_iter().enumerate().collect::<Vec<_>>();
    indexed.sort_by_key(|(index, cell)| {
        (
            Reverse(*feedback_scores.get(&cell.cell_id).unwrap_or(&0)),
            *index,
        )
    });
    indexed.into_iter().map(|(_, cell)| cell).collect()
}

pub fn estimate_tokens(payload: &[u8]) -> u32 {
    if payload.is_empty() {
        return 0;
    }
    let bytes = match u32::try_from(payload.len()) {
        Ok(value) => value,
        Err(_) => return u32::MAX,
    };
    bytes.saturating_add(3) / 4
}

fn effective_budget(view: &AgentView, requested: u32) -> u32 {
    let budget = if requested == 0 {
        view.default_context_budget_tokens
    } else {
        requested
    };
    view.effective_budget(budget)
}

fn extract_citation(payload: &[u8]) -> Option<String> {
    CellMetadata::from_payload(payload)
        .citation()
        .map(str::to_owned)
}

fn effective_redundancy_threshold(value: u16) -> u16 {
    if value == 0 {
        DEFAULT_REDUNDANCY_THRESHOLD_Q16
    } else {
        value
    }
}

fn cosine_similarity_q16(u: &[i16], v: &[i16]) -> u16 {
    if u.len() != v.len() || u.is_empty() {
        return 0;
    }
    let mut dot_product = 0.0;
    let mut norm_u = 0.0;
    let mut norm_v = 0.0;
    for i in 0..u.len() {
        let ui = u[i] as f64;
        let vi = v[i] as f64;
        dot_product += ui * vi;
        norm_u += ui * ui;
        norm_v += vi * vi;
    }
    if norm_u == 0.0 || norm_v == 0.0 {
        return 0;
    }
    let similarity = dot_product / (norm_u.sqrt() * norm_v.sqrt());
    if similarity <= 0.0 {
        0
    } else {
        (similarity * 65536.0).min(65535.0) as u16
    }
}

fn is_redundant(payload: &[u8], packed: &[ContextPackCell], threshold_q16: u16) -> bool {
    if let Some(current_vec) = crate::search::vector::vector_from_payload(payload) {
        return packed.iter().any(|cell| {
            if let Some(cell_vec) = crate::search::vector::vector_from_payload(&cell.payload) {
                cosine_similarity_q16(&current_vec, &cell_vec) >= threshold_q16
            } else {
                false
            }
        });
    }

    let current = term_set(payload);
    if current.is_empty() {
        return false;
    }
    packed
        .iter()
        .any(|cell| weighted_jaccard_q16(&current, &term_set(&cell.payload)) >= threshold_q16)
}

fn term_set(payload: &[u8]) -> BTreeSet<String> {
    let metadata = CellMetadata::from_payload(payload);
    tokenize(&metadata.body_text).into_iter().collect()
}

fn weighted_jaccard_q16(left: &BTreeSet<String>, right: &BTreeSet<String>) -> u16 {
    if left.is_empty() || right.is_empty() {
        return 0;
    }
    let intersection = left.intersection(right).count() as u64;
    let union = left.union(right).count() as u64;
    ((intersection * 65_535 + union / 2) / union) as u16
}
