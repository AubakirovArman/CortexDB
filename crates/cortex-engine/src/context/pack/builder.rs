use std::collections::{BTreeMap, BTreeSet};

use cortex_aql::AgentView;
use cortex_core::CellId;

use super::access::context_access_decision;
use super::ordering::diversity_aware_order;
use crate::context::dedup::{
    effective_redundancy_threshold, is_redundant, term_set, weighted_jaccard_q16,
};
use crate::context::explain::{extract_query_terms, generate_selection_reason};
use crate::context::freshness::{SourceFreshness, SourceFreshnessRange};
use crate::context::large_cell::{
    apply_large_cell_policy, estimate_cell_tokens, ContextLargeCellPolicy, LargeCellDecision,
    LargeCellRequest,
};
use crate::context::scoring::{apply_feedback_bonus, score_components};
use crate::context::span::{select_relevant_span, ContextSpanRequest};
use crate::context::{
    answerability, conflicts, ContextExplain, ContextPack, ContextPackAnomaly,
    ContextPackAnomalyCode, ContextPackCell, ContextPackOptions,
};
use crate::database::RetrievedCell;
use crate::search::tokenize;
use crate::source_trust::SourceTrust;

impl ContextPack {
    pub fn from_retrieved_with_feedback_options_and_view(
        cells: Vec<RetrievedCell>,
        token_budget_tokens: u32,
        citations_required: bool,
        options: &ContextPackOptions,
        query: &str,
        feedback_scores: &BTreeMap<CellId, i32>,
        access_view: Option<&AgentView>,
    ) -> Self {
        let mut pack_cells = Vec::new();
        let mut estimated_tokens = 0u32;
        let mut truncated = false;
        let mut anomalies = Vec::new();
        let query_terms = extract_query_terms(query);
        let source_freshness_range = SourceFreshnessRange::from_cells(&cells);
        let cells = if options.reduce_redundancy {
            diversity_aware_order(cells, &query_terms)
        } else {
            cells
        };

        for cell in cells {
            let metadata = cell.metadata();
            let citation = metadata.citation().map(str::to_owned);
            let access_decision = context_access_decision(cell.cell_id, &metadata, access_view);
            let cell_body_terms = tokenize(&metadata.body_text)
                .into_iter()
                .collect::<BTreeSet<_>>();
            if options.reduce_redundancy
                && is_redundant(
                    &cell.payload,
                    &metadata,
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

            let mut selected_payload = cell.payload;
            let mut selected_tokens = estimate_cell_tokens(
                &selected_payload,
                citation.as_deref(),
                citations_required,
                options,
            );
            let mut selected_provenance = None;
            let remaining_tokens = token_budget_tokens.saturating_sub(estimated_tokens);
            if let Some(selection) = select_relevant_span(ContextSpanRequest {
                cell_id: cell.cell_id,
                payload: &selected_payload,
                citation: citation.as_deref(),
                citations_required,
                original_tokens: selected_tokens,
                remaining_tokens,
                query_terms: &query_terms,
                options,
                source_ref: metadata.source_ref.as_ref(),
            }) {
                truncated = true;
                anomalies.push(selection.anomaly);
                selected_payload = selection.payload;
                selected_tokens = selection.estimated_tokens;
                selected_provenance = Some(selection.provenance);
            }
            let would_exceed = selected_tokens > remaining_tokens;
            if would_exceed
                && (options.large_cell_policy != ContextLargeCellPolicy::PreserveFirst
                    || !pack_cells.is_empty())
            {
                truncated = true;
                if options.large_cell_policy == ContextLargeCellPolicy::PreserveFirst {
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
                match apply_large_cell_policy(LargeCellRequest {
                    cell_id: cell.cell_id,
                    payload: &selected_payload,
                    citation: citation.as_deref(),
                    citations_required,
                    original_tokens: selected_tokens,
                    remaining_tokens,
                    options,
                }) {
                    LargeCellDecision::Include {
                        payload,
                        estimated_tokens,
                        anomaly,
                    } => {
                        anomalies.push(anomaly);
                        selected_payload = payload;
                        selected_tokens = estimated_tokens;
                    }
                    LargeCellDecision::Exclude { anomaly } => {
                        anomalies.push(anomaly);
                        continue;
                    }
                }
            }
            if pack_cells.is_empty() && selected_tokens > token_budget_tokens {
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
            let source_trust =
                SourceTrust::from_metadata(metadata.source_trust_q16, metadata.source_trust_class);
            let source_trust_bonus = source_trust.score_bonus();
            let source_freshness = SourceFreshness::from_created_unix_seconds(
                metadata.created_unix_seconds,
                source_freshness_range,
            );
            let source_freshness_bonus = source_freshness.score_bonus();

            let mut max_jaccard_similarity_q16 = 0u32;
            for packed in &pack_cells {
                let jaccard =
                    weighted_jaccard_q16(&cell_body_terms, &term_set(&packed.metadata)) as u32;
                if jaccard > max_jaccard_similarity_q16 {
                    max_jaccard_similarity_q16 = jaccard;
                }
            }
            let redundancy_penalty = (max_jaccard_similarity_q16 * 10_000) / 65536;

            let base_score = base_bm25
                .saturating_add(source_trust_bonus)
                .saturating_add(source_freshness_bonus)
                .saturating_sub(redundancy_penalty);
            let feedback_bonus = *feedback_scores.get(&cell.cell_id).unwrap_or(&0);
            let score = apply_feedback_bonus(base_score, feedback_bonus);

            let why_selected =
                generate_selection_reason(score, base_bm25, source_trust_bonus, redundancy_penalty);
            let score_components = score_components(
                base_bm25,
                source_trust,
                source_freshness,
                redundancy_penalty,
                feedback_bonus,
            );

            let explain = Some(ContextExplain {
                score,
                matched_terms: matched,
                why_selected,
                score_components,
                base_bm25,
                source_trust_q16: source_trust.q16,
                source_trust_category: source_trust.category,
                source_trust_bonus,
                source_freshness_q16: source_freshness.q16,
                source_freshness_category: source_freshness.category,
                source_freshness_bonus,
                redundancy_penalty,
            });

            estimated_tokens = estimated_tokens.saturating_add(selected_tokens);
            pack_cells.push(ContextPackCell {
                cell_id: cell.cell_id,
                payload: selected_payload,
                metadata,
                estimated_tokens: selected_tokens,
                citation,
                provenance: selected_provenance,
                explain,
                access_decision: Some(access_decision),
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
