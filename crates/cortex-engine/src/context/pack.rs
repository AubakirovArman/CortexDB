use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet};

use cortex_aql::{AgentView, BoundPlan};
use cortex_core::CellId;

use super::answerability;
use super::conflicts;
use super::dedup::{effective_redundancy_threshold, is_redundant, term_set, weighted_jaccard_q16};
use super::explain::{extract_query_terms, generate_selection_reason};
use super::{
    ContextExplain, ContextPack, ContextPackAnomaly, ContextPackAnomalyCode, ContextPackCell,
    ContextPackOptions, ContextScoreComponent,
};
use crate::database::{Database, RetrievedCell};
use crate::error::{EngineError, EngineResult};
use crate::query::{cache::AqlStatementKind, CellMetadata, EngineAqlProvider};
use crate::search::tokenize;
use crate::source_trust::SourceTrust;

impl Database {
    /// Compile a `RETRIEVE CONTEXT` AQL statement into a scored ContextPack.
    ///
    /// The same bound retrieve plan drives bitmap filtering, AQL `LIMIT`,
    /// `BUDGET`, `REQUIRE` gates, and the ContextPack citation policy.
    pub fn context_pack_from_aql(
        &self,
        aql: &str,
        view: &AgentView,
        options: ContextPackOptions,
    ) -> EngineResult<ContextPack> {
        let (cached, index) = self.bind_aql_cached(aql, view)?;
        if cached.statement_kind != AqlStatementKind::Retrieve {
            return Err(EngineError::InvalidOperation);
        }
        let BoundPlan::Retrieve(plan) = cached.bound_plan else {
            return Err(EngineError::InvalidOperation);
        };
        let provider = EngineAqlProvider::new(index, view);
        let budget = effective_budget(
            view,
            options.token_budget_tokens,
            plan.context_policy.budget_tokens,
        );
        let citations_required = options.require_citations || plan.context_policy.require_citations;
        let cells = order_by_feedback(
            self.retrieve_cells(&plan, &provider)?,
            &self.feedback_scores(),
        );
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
        let query_terms = extract_query_terms(query);

        for cell in cells {
            let citation = extract_citation(&cell.payload);
            let metadata = CellMetadata::from_payload(&cell.payload);
            let cell_body_terms = tokenize(&metadata.body_text)
                .into_iter()
                .collect::<BTreeSet<_>>();
            if options.reduce_redundancy
                && is_redundant(
                    &cell.payload,
                    &pack_cells,
                    effective_redundancy_threshold(options.redundancy_threshold_q16),
                )
            {
                anomalies.push(ContextPackAnomaly {
                    cell_id: Some(cell.cell_id),
                    code: ContextPackAnomalyCode::RedundantCell,
                    message: "selected cell is redundant with an earlier packed cell".to_owned(),
                    why_excluded: Some(
                        "excluded because reduce_redundancy is enabled and similarity exceeds the configured threshold"
                            .to_owned(),
                    ),
                });
                continue;
            }

            let cell_tokens = estimate_cell_tokens(
                &cell.payload,
                citation.as_deref(),
                citations_required,
                options,
            );
            let would_exceed = !pack_cells.is_empty()
                && estimated_tokens.saturating_add(cell_tokens) > token_budget_tokens;
            if would_exceed {
                truncated = true;
                anomalies.push(ContextPackAnomaly {
                    cell_id: Some(cell.cell_id),
                    code: ContextPackAnomalyCode::TokenOverload,
                    message: "candidate would exceed the remaining token budget".to_owned(),
                    why_excluded: Some(
                        "excluded because estimated_tokens would exceed token_budget_tokens; skipped so later smaller candidates can still fit"
                            .to_owned(),
                    ),
                });
                continue;
            }
            if pack_cells.is_empty() && cell_tokens > token_budget_tokens {
                truncated = true;
            }

            if citations_required && citation.is_none() {
                anomalies.push(ContextPackAnomaly {
                    cell_id: Some(cell.cell_id),
                    code: ContextPackAnomalyCode::MissingCitation,
                    message: "selected cell does not include source= or citation=".to_owned(),
                    why_excluded: None,
                });
            }

            let matched: Vec<String> = query_terms
                .iter()
                .filter(|term| cell_body_terms.contains(*term))
                .cloned()
                .collect();

            let base_bm25 = (matched.len() as u32) * 10_000;
            let source_trust = SourceTrust::from_q16(metadata.source_trust_q16);
            let source_trust_bonus = source_trust.score_bonus();

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

            let why_selected =
                generate_selection_reason(score, base_bm25, source_trust_bonus, redundancy_penalty);
            let score_components = score_components(base_bm25, source_trust, redundancy_penalty);

            let explain = Some(ContextExplain {
                score,
                matched_terms: matched,
                why_selected,
                score_components,
                base_bm25,
                source_trust_q16: source_trust.q16,
                source_trust_category: source_trust.category,
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

        let answerability = answerability::estimate(&pack_cells, &query_terms);
        let conflict_visibility = conflicts::measure(&pack_cells);
        if answerability.is_insufficient() {
            anomalies.push(answerability::insufficient_context_anomaly(&answerability));
        }

        Self {
            cells: pack_cells,
            token_budget_tokens,
            estimated_tokens,
            truncated,
            citations_required,
            answerability_q16: answerability.score_q16,
            conflict_visibility_q16: conflict_visibility.conflict_visibility_q16,
            visible_conflict_count: conflict_visibility.visible_conflict_count,
            anomalies,
        }
    }
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

fn score_components(
    base_bm25: u32,
    source_trust: SourceTrust,
    redundancy_penalty: u32,
) -> Vec<ContextScoreComponent> {
    let source_trust_bonus = source_trust.score_bonus();
    vec![
        ContextScoreComponent {
            name: "base_bm25".to_owned(),
            value: base_bm25,
            contribution: i32::try_from(base_bm25).unwrap_or(i32::MAX),
            reason: "keyword overlap between query terms and cell body".to_owned(),
        },
        ContextScoreComponent {
            name: "source_trust_bonus".to_owned(),
            value: source_trust_bonus,
            contribution: i32::try_from(source_trust_bonus).unwrap_or(i32::MAX),
            reason: source_trust.score_reason(),
        },
        ContextScoreComponent {
            name: "redundancy_penalty".to_owned(),
            value: redundancy_penalty,
            contribution: -i32::try_from(redundancy_penalty).unwrap_or(i32::MAX),
            reason: "weighted Jaccard overlap with already packed cells".to_owned(),
        },
    ]
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

fn estimate_cell_tokens(
    payload: &[u8],
    citation: Option<&str>,
    citations_required: bool,
    options: &ContextPackOptions,
) -> u32 {
    let citation_overhead = if citations_required && citation.is_some() {
        options.citation_overhead_tokens
    } else {
        0
    };
    estimate_tokens(payload).saturating_add(citation_overhead)
}

fn effective_budget(view: &AgentView, requested: u32, plan_budget: u32) -> u32 {
    let budget = if requested == 0 {
        plan_budget
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
