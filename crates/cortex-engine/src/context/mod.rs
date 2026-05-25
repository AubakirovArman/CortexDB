use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet};

use cortex_aql::AgentView;
use cortex_core::CellId;

use crate::database::{Database, RetrievedCell};
use crate::error::EngineResult;
use crate::query::CellMetadata;
use crate::search::tokenize;

pub mod dedup;
pub mod explain;

use dedup::{effective_redundancy_threshold, is_redundant, term_set, weighted_jaccard_q16};
use explain::extract_query_terms;

pub const DEFAULT_REDUNDANCY_THRESHOLD_Q16: u16 = 32_768;

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
pub struct ContextExplain {
    pub score: u32,
    pub matched_terms: Vec<String>,
    pub why_selected: String,
    pub base_bm25: u32,
    pub source_trust_bonus: u32,
    pub redundancy_penalty: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContextPackCell {
    pub cell_id: CellId,
    pub payload: Vec<u8>,
    pub estimated_tokens: u32,
    pub citation: Option<String>,
    pub explain: Option<ContextExplain>,
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
            aql,
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
            "",
        )
    }

    pub fn from_retrieved_with_options(
        cells: Vec<RetrievedCell>,
        token_budget_tokens: u32,
        citations_required: bool,
        options: &ContextPackOptions,
        query: &str,
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

            let query_terms = extract_query_terms(query);
            let metadata = CellMetadata::from_payload(&cell.payload);
            let cell_body_terms = tokenize(&metadata.body_text)
                .into_iter()
                .collect::<BTreeSet<_>>();
            let matched: Vec<String> = query_terms
                .iter()
                .filter(|term| cell_body_terms.contains(*term))
                .cloned()
                .collect();

            let base_bm25 = (matched.len() as u32) * 10_000;
            let source_trust_bonus = metadata.source_trust_q16.unwrap_or(32768) as u32;

            let mut max_jaccard_similarity_q16 = 0u32;
            for packed in &pack_cells {
                let jaccard =
                    weighted_jaccard_q16(&cell_body_terms, &term_set(&packed.payload)) as u32;
                if jaccard > max_jaccard_similarity_q16 {
                    max_jaccard_similarity_q16 = jaccard;
                }
            }
            let redundancy_penalty = (max_jaccard_similarity_q16 * 10_000) / 65536;

            let score = base_bm25
                .saturating_add(source_trust_bonus)
                .saturating_sub(redundancy_penalty);

            let why_selected = if redundancy_penalty > 5000 {
                "contains relevant terms but heavily penalised for semantic redundancy".to_owned()
            } else if source_trust_bonus > 40000 {
                "highest semantic relevance and highly trusted source provenance".to_owned()
            } else {
                "contains relevant matched terms with standard provenance trust".to_owned()
            };

            let explain = Some(ContextExplain {
                score,
                matched_terms: matched,
                why_selected,
                base_bm25,
                source_trust_bonus,
                redundancy_penalty,
            });

            estimated_tokens = estimated_tokens.saturating_add(cell_tokens);
            pack_cells.push(ContextPackCell {
                cell_id: cell.cell_id,
                payload: cell.payload,
                estimated_tokens: cell_tokens,
                citation,
                explain,
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
