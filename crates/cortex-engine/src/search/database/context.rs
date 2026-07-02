use std::collections::BTreeSet;

use cortex_aql::AgentView;

use crate::context::{ContextPack, ContextPackAnomaly, ContextPackAnomalyCode, ContextPackOptions};
use crate::database::{Database, RetrievedCell};
use crate::feedback::current_unix_seconds;
use crate::query::CellMetadata;

use super::super::ann::{AnnFallbackReason, AnnSearchReport, AnnSloViolation};
use super::DatabaseSearchOutcome;

impl Database {
    #[allow(clippy::too_many_arguments)]
    pub fn context_pack_from_search_outcome_with_options(
        &self,
        outcome: DatabaseSearchOutcome,
        view: &AgentView,
        token_budget_tokens: u32,
        citations_required: bool,
        options: &ContextPackOptions,
        query: &str,
    ) -> ContextPack {
        let initial_anomalies = retrieval_incomplete_anomalies(outcome.ann_report.as_ref());
        let cells = outcome
            .results
            .into_iter()
            .filter_map(|result| {
                self.get_latest_cell_with_descriptor(result.cell_id)
                    .map(|(payload, descriptor)| RetrievedCell {
                        cell_id: result.cell_id,
                        payload,
                        descriptor,
                        captured_access_decision: None,
                    })
            })
            .collect::<Vec<_>>();
        let feedback_scores = self.feedback_scores_for_cells_at(
            cells.iter().map(|cell| cell.cell_id),
            current_unix_seconds(),
        );

        ContextPack::from_retrieved_with_feedback_options_view_and_anomalies(
            cells,
            token_budget_tokens,
            citations_required,
            options,
            query,
            &feedback_scores,
            Some(view),
            initial_anomalies,
        )
    }
}

fn retrieval_incomplete_anomalies(report: Option<&AnnSearchReport>) -> Vec<ContextPackAnomaly> {
    let Some(report) = report else {
        return Vec::new();
    };
    if report.fallback_reason != Some(AnnFallbackReason::VisitBudgetExceeded)
        && !report
            .slo_violations
            .contains(&AnnSloViolation::VisitBudgetExceeded)
    {
        return Vec::new();
    }

    vec![ContextPackAnomaly {
        cell_id: None,
        code: ContextPackAnomalyCode::RetrievalIncomplete,
        message: "retrieval may be incomplete because the ANN visit budget was exhausted"
            .to_owned(),
        why_excluded: Some(
            "ANN traversal exhausted max_visited_candidates before proving complete retrieval"
                .to_owned(),
        ),
    }]
}

pub(super) fn search_parent_lookup_keys(metadata: &CellMetadata) -> Vec<String> {
    let mut keys = BTreeSet::new();
    if let Some(parent_id) = &metadata.parent_id {
        keys.insert(parent_id.clone());
    }
    if !is_search_parent_context_metadata(metadata) {
        if let Some(document_id) = &metadata.document_id {
            keys.insert(document_id.clone());
        }
    }
    keys.into_iter().collect()
}

pub(super) fn is_search_parent_context_metadata(metadata: &CellMetadata) -> bool {
    metadata
        .chunk_role
        .as_deref()
        .map(|role| {
            role.eq_ignore_ascii_case("parent")
                || role.eq_ignore_ascii_case("document")
                || role.eq_ignore_ascii_case("summary")
        })
        .unwrap_or(false)
}

pub(super) fn high_level_anchor_score(metadata: &CellMetadata) -> u64 {
    let mut score = 0u64;
    if is_search_parent_context_metadata(metadata) {
        score = score.saturating_add(8_000);
    }
    for value in [
        metadata.title.as_deref(),
        metadata.path.as_deref(),
        metadata.document_id.as_deref(),
        metadata.source.as_deref(),
        Some(metadata.body_text.as_str()),
    ]
    .into_iter()
    .flatten()
    {
        let value = value.to_ascii_lowercase();
        for term in [
            "overview", "summary", "mission", "charter", "about", "strategy", "vision", "company",
        ] {
            if value.contains(term) {
                score = score.saturating_add(2_000);
            }
        }
    }
    score
}

pub(super) fn project_context_score(metadata: &CellMetadata) -> u64 {
    let mut score = 1_000u64;
    if metadata.owner.is_some() {
        score = score.saturating_add(2_000);
    }
    if metadata.status_tag.is_some() {
        score = score.saturating_add(1_500);
    }
    if metadata.event_date.is_some() {
        score = score.saturating_add(1_000);
    }
    if metadata.title.is_some() {
        score = score.saturating_add(500);
    }
    score
}
